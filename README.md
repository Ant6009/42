# 42

A standalone, local-network [Perplexity](https://www.perplexity.ai)-style answer engine.
Ask a question, get a streamed answer with inline citations from live web search,
backed by any OpenAI-compatible LLM endpoint you already run.

42 is **pure glue**: it ships no LLM, no search index, no frontend build step.
It connects three things you already have on your LAN:

- an **OpenAI-compatible chat endpoint** (llama.cpp server, Ollama, LM Studio, ...)
- a **SearXNG** instance (JSON API)
- a browser

Everything else — accounts, conversations, citations, streaming — is one small
static Rust binary.

## Features

- **Cited answers**: the top search results (title, URL, snippet) are injected into
  the prompt; the model answers with inline `[n]` markers; the backend validates and
  repairs markers before they reach you, and shows you the actual sources.
- **Streaming**: Server-Sent Events end to end. Thinking models (e.g. Qwen3) emit a
  `thinking` phase so you see progress instead of a frozen page.
- **Multi-turn conversations**: full history is re-sent each turn (re-searched too),
  truncated by a sliding window and a token budget when the context window is small.
- **Per-user accounts**: argon2id passwords, HTTP-only cookie sessions, per-user
  conversation history. Users are provisioned by an admin via CLI.
- **Zero build-step frontend**: vanilla HTML/CSS/JS embedded in the binary with
  `include_str!`. One file to deploy.
- **One binary, one config file, one SQLite database** (WAL mode).

## Architecture

```
browser ──HTTPS/HTTP──▶ 42 (axum)
                          │  ├── SQLite: users, sessions, conversations, messages
                          │  ├── SearXNG JSON API ──▶ top-N results + snippets
                          │  └── OpenAI-compatible /chat/completions (SSE)
                          │        (llama.cpp / Ollama / LM Studio)
                          ▼
              prompt = system + sources + history window + question
              stream → validate [n] citations → persist → SSE to browser
```

Request flow for `POST /v1/ask`:

1. Load the conversation (or create one).
2. Search SearXNG for the question; keep the top `max_results`.
3. Build the prompt: system instructions, numbered sources, the last
   `history_window` turns within `context_window` tokens, then the new question.
4. Stream the completion. Content tokens are forwarded as `token` events;
   `reasoning_content` (thinking models) is forwarded as `thinking` events.
5. Validate/repair `[n]` citation markers against the real source list.
6. Persist the user and assistant messages; emit `done`.

## Quick start (NixOS)

42 ships a NixOS module. In your `/etc/nixos` flake:

```nix
inputs.fortytwo = {
  url = "github:Ant6009/42";
  inputs.nixpkgs.follows = "nixpkgs";
};

outputs = { ... , fortytwo }: {
  nixosConfigurations.yourhost = lib.nixosSystem {
    modules = [
      fortytwo.nixosModules.default
      {
        services.fortytwo = {
          enable = true;
          configPath = /etc/42/42.toml; # see below
        };
      }
    ];
  };
};
```

Write `/etc/42/42.toml`:

```toml
[server]
bind = "0.0.0.0:4242"

[llm]
base_url = "http://127.0.0.1:9292/v1"   # llama.cpp server
model = "your-model"
context_window = 32768

[search]
base_url = "http://127.0.0.1:8888"      # SearXNG
max_sources = 8
snippet_chars = 500

[database]
path = "/var/lib/42/42.sqlite"
history_turns = 10

# Optional: seed an admin account on first start (before the DB exists).
# Generate a hash with:  42 admin hash-password
# [admin]
# username = "admin"
# password_hash = "$argon2id$v=19$..."
```

Then `nixos-rebuild switch` and open `http://<host>:4242`.

No Nix? The flake also builds a plain package:

```sh
nix build github:Ant6009/42            # or: nix build .
./result/bin/42 --config 42.toml serve
```

## Configuration

The TOML file has two roles: it sets the things that cannot change at runtime
(`server.bind`, `db.path`), and it provides the **initial** engine settings, which
are seeded into the database on first start. Afterwards the database is the
source of truth and admins edit the engine settings live from the UI (gear icon
in the sidebar) — no restart needed.

| Key | Default | Description |
| --- | --- | --- |
| `server.bind` | `127.0.0.1:4242` | Listen address. |
| `database.path` | `42.sqlite` | SQLite file (WAL mode). |
| `llm.base_url` | — | OpenAI-compatible base URL (must end in `/v1` for llama.cpp/Ollama). |
| `llm.model` | — | Model name passed to the endpoint. |
| `llm.context_window` | `32768` | Rough token budget used to trim history. |
| `search.base_url` | — | SearXNG base URL (JSON API enabled). |
| `search.max_sources` | `8` | Results injected into the prompt (5–8 works well). |
| `search.snippet_chars` | `500` | Per-snippet character budget. |
| `database.history_turns` | `10` | Max past turns re-sent to the model. |
| `admin.username` / `admin.password_hash` | — | Seed an admin when the DB is new. |

Everything except `server.bind` and `database.path` is editable at runtime via
`GET/PUT /v1/settings` (admin only) or the settings page in the UI.

## HTTP API

All endpoints live under `/v1` and (except login) require the session cookie.

| Method & path | Purpose |
| --- | --- |
| `POST /v1/auth/login` | `{"username","password"}` → sets `42_session` cookie. |
| `POST /v1/auth/logout` | Clears the session. |
| `GET /v1/me` | Current user. |
| `POST /v1/ask` | `{"conversation_id"?, "question"}` → SSE stream. |
| `GET/POST /v1/conversations` | List / create conversations. |
| `GET/DELETE /v1/conversations/{id}` | Fetch (with messages) / delete. |

SSE events from `/v1/ask`:

```
data: {"type":"conversation","id":"..."}
data: {"type":"sources","sources":[{"index":1,"title":"...","url":"...","snippet":"..."}]}
data: {"type":"thinking","text":"..."}   # only for thinking models
data: {"type":"token","text":"..."}
data: {"type":"final","text":"..."}      # only if citation validation changed the answer
data: {"type":"done"}
data: {"type":"error","message":"..."}
```

## CLI

```sh
42 serve --config 42.toml              # run the server
42 admin hash-password                 # read a password, print an argon2id hash
42 admin create-user alice [--is-admin] # create an account (double-entry password)
```

## Development

```sh
devenv shell cargo test      # unit + integration tests (mock servers)
devenv shell cargo clippy
```

Layout: `src/server` (HTTP, SSE), `src/engine` (search, prompt, citations, LLM
client), `src/auth`, `src/store` (SQLite), `src/web` (embedded UI).

## Design notes

- **Citations are enforced, not trusted**: the model is told to cite with `[n]`,
  but every marker is checked against the real source list afterwards — unknown
  indices are dropped, and a zero-citation answer to a factual question is
  repaired conservatively. The `final` event carries the corrected text.
- **Thinking models are first-class**: `reasoning_content` chunks are streamed as
  `thinking` events and never persisted or cited.
- **No bundling by design**: 42 never downloads models, runs search engines, or
  calls cloud APIs. Point it at what you run; swap backends by editing TOML.
- **Upgrade paths kept open**: tokens carry a `kind` column (session/bearer) and
  user creation lives in the service layer, so bearer tokens and self-registration
  can be added without a schema rewrite.

## Status

v1: single binary, web UI + API, accounts, conversations, cited streaming answers.
Deferred: bearer tokens / API keys, self-registration, per-user settings, model
picker, cloud LLM providers.
