# Grill Me Results

Generated: 2026-09-13T10:45:09.528Z

## Plan

(state was created by grill_record_turn; no plan recorded)

## Shared Understanding

42 is a standalone, local-network Perplexity clone: a Rust/axum service running as a NixOS systemd service on 192.168.68.128, talking to a local llama.cpp server (OpenAI-compatible) and a local SearXNG instance. Bundles nothing; pure glue. First-class features: per-user accounts, multi-turn cited answers with streaming, per-user conversation history.

## Questions and Answers

### 1. Which LLM backend should 42 use for answer generation: local only (Ollama/llama.cpp), cloud APIs, or a provider abstraction supporting both?

**Recommended answer:** C (both, via provider abstraction) if the goal is flexibility; A (local only) if genuinely offline is the point.

**User answer:** _(not recorded)_

**Status:** open

**Notes:** First question of the 42 product requirements interview. Empty project dir at /home/antoine/piProjects/42.

### 2. Which LLM backend should 42 use for answer generation?

**Recommended answer:** C (both) or A (local only)

**User answer:** A — local only for now (Ollama/llama.cpp); C (provider abstraction) may come in the future.

**Status:** resolved

**Notes:** Local-first. Design should not hardwire one runtime if cheap to avoid.

### 3. Which local model runtime does 42 integrate with: generic OpenAI-compatible endpoint, Ollama specifically, or llama.cpp specifically?

**Recommended answer:** A — generic OpenAI-compatible HTTP endpoint (base_url + model), covers Ollama/llama.cpp/LM Studio, keeps future cloud support cheap.

**User answer:** _(not recorded)_

**Status:** open

### 4. Which local model runtime does 42 integrate with?

**Recommended answer:** A — generic OpenAI-compatible endpoint.

**User answer:** A — generic OpenAI-compatible HTTP endpoint.

**Status:** resolved

**Notes:** Config: base_url + model. No model management inside 42.

### 5. Where does 42 get web search results: existing SearXNG instance, paid search API, or direct scraping?

**Recommended answer:** A — existing SearXNG instance via its JSON API, configured by URL.

**User answer:** _(not recorded)_

**Status:** open

**Notes:** 42 bundles nothing; search backend must live outside the service.

### 6. Where does 42 get web search results?

**Recommended answer:** A — existing SearXNG instance via JSON API.

**User answer:** A — existing SearXNG instance; URL provided via config at deployment.

**Status:** resolved

**Notes:** Instance address to be pinned in NixOS config when we get to packaging.

### 7. What is 42's interface: web UI, API-only, or web UI plus a clean HTTP API?

**Recommended answer:** C — web UI plus a clean HTTP API on the same service.

**User answer:** _(not recorded)_

**Status:** open

### 8. What is 42's interface?

**Recommended answer:** C — web UI plus clean HTTP API.

**User answer:** C — web UI plus a clean HTTP API on the same service.

**Status:** resolved

### 9. What implementation language/stack for 42?

**Recommended answer:** A — Rust backend (single static binary, clean Nix service) with a small static frontend.

**User answer:** _(not recorded)_

**Status:** open

### 10. What implementation language/stack for 42?

**Recommended answer:** A — Rust backend with small static frontend.

**User answer:** A — Rust.

**Status:** resolved

### 11. How is the 42 frontend built and embedded: no-build-step vanilla JS in the binary, or a Vite/TS build in the Nix derivation?

**Recommended answer:** A — hand-written HTML/CSS/vanilla JS embedded via include_str!, no build step.

**User answer:** _(not recorded)_

**Status:** open

### 12. How is the 42 frontend built and embedded?

**Recommended answer:** A — no build step, vanilla JS embedded in binary.

**User answer:** A — no build step.

**Status:** resolved

### 13. What features are in 42 v1: multi-turn, citations, streaming, search-only mode, history, model picker?

**Recommended answer:** v1 = a (multi-turn) + b (citations) + c (streaming) + e (history); defer d (search-only) and f (model picker).

**User answer:** _(not recorded)_

**Status:** open

### 14. What features are in 42 v1?

**Recommended answer:** a+b+c+e (multi-turn, citations, streaming, history).

**User answer:** a, b, c — multi-turn conversation, inline citations, streaming. No history (e), no search-only mode (d), no model picker (f) in v1.

**Status:** resolved

### 15. How are citations produced: prompt-driven with validation, or post-hoc attribution? And how many sources injected per turn?

**Recommended answer:** A — prompt-driven [n] markers with backend validation; top 5-8 sources, snippets ~500 chars.

**User answer:** _(not recorded)_

**Status:** open

### 16. How are citations produced?

**Recommended answer:** A — prompt-driven markers with validation; 5-8 sources, ~500 char snippets.

**User answer:** A, with 5-8 sources and ~500 char snippets — agreed.

**Status:** resolved

**Notes:** Backend drops citation markers that don't match the source list.

### 17. Does 42 re-run search on every turn or only the first? And how is history bounded for local model context limits?

**Recommended answer:** Re-search every turn; sliding window of last K turns (default 10, configurable), no summarization in v1.

**User answer:** _(not recorded)_

**Status:** open

### 18. Does 42 re-run search every turn, and how is history bounded?

**Recommended answer:** Re-search every turn + sliding window of last K turns (default 10).

**User answer:** A — re-search every turn, plus sliding window (K=10 default, configurable), no summarization in v1.

**Status:** resolved

### 19. How is 42 packaged/configured under Nix: systemd service with TOML config file, or a full NixOS module (services.42)?

**Recommended answer:** A — derivation + systemd service + TOML config on disk for v1; NixOS module later if desired.

**User answer:** _(not recorded)_

**Status:** open

### 20. How is 42 packaged/configured under Nix?

**Recommended answer:** A — derivation + systemd + TOML config.

**User answer:** A — derivation + systemd service + TOML config file on disk.

**Status:** resolved

### 21. Streaming protocol and API shape: SSE with POST /v1/ask emitting sources/token/done events, localhost-only, no auth?

**Recommended answer:** SSE; POST /v1/ask {conversation_id, question} -> SSE events (sources, token, done); bind 127.0.0.1 default, no auth.

**User answer:** _(not recorded)_

**Status:** open

### 22. Streaming protocol and API shape?

**Recommended answer:** SSE; POST /v1/ask; localhost-only, no auth.

**User answer:** A — SSE, with the proposed /v1/ask API shape and localhost-only default.

**Status:** resolved

### 23. Where does conversation state live: in-memory or SQLite?

**Recommended answer:** A — in-memory for v1, behind a ConversationStore trait so SQLite can be swapped in later.

**User answer:** _(not recorded)_

**Status:** open

### 24. Where does conversation state live?

**Recommended answer:** A — in-memory behind a trait.

**User answer:** B — SQLite from day one.

**Status:** resolved

**Notes:** Store behind a ConversationStore trait so the engine stays decoupled. SQLite file path goes in the TOML config.

### 25. Which Rust crates for the 42 backend?

**Recommended answer:** axum + tokio + reqwest + rusqlite (via spawn_blocking) + serde + tower-http + tracing.

**User answer:** _(not recorded)_

**Status:** open

### 26. Which Rust crates for the 42 backend?

**Recommended answer:** axum + tokio + reqwest + rusqlite + serde + tower-http + tracing.

**User answer:** Accepted the proposed stack.

**Status:** resolved

### 27. Are the discovered endpoints (llama.cpp at 192.168.68.128:9292/v1, SearXNG at 192.168.68.128:8888) the right defaults, and should the backend enforce a configurable context window by truncating oldest turns?

**Recommended answer:** Yes to both; context_window default 32768, truncate oldest history first.

**User answer:** _(not recorded)_

**Status:** open

**Notes:** Discovered from pi-llama-swap.json and the searXNG pi extension. Model: Qwen3.8-27b-UD-Q6_K_M.

### 28. Default endpoints and context budgeting?

**Recommended answer:** Yes to both.

**User answer:** Endpoints confirmed, but 42 itself runs ON 192.168.68.128 as a service for the whole local network (not on this laptop). Context window enforcement: yes, configurable, default 32768, truncate oldest turns first.

**Status:** resolved

**Notes:** Revises Q11's localhost-only assumption: 42 is LAN-facing. Auth/binding question reopened.

### 29. Auth/binding for the LAN-facing 42 service: open, optional shared token, or accounts?

**Recommended answer:** B — bind 0.0.0.0 with an optional shared token, enabled by default with a generated token.

**User answer:** _(not recorded)_

**Status:** open

### 30. Auth/binding for the LAN-facing 42 service?

**Recommended answer:** B — optional shared token.

**User answer:** C — per-user accounts. This is a big part of the service, not an afterthought.

**Status:** resolved

**Notes:** Big scope addition: auth is a first-class feature. History (per-user) likely follows.

### 31. How do user accounts work: cookie sessions vs bearer tokens, and open registration vs admin-provisioned?

**Recommended answer:** A + b — argon2 passwords, HTTP-only cookie sessions, admin-provisioned accounts; token table designed so bearer tokens can be added later.

**User answer:** _(not recorded)_

**Status:** open

### 32. How do user accounts work?

**Recommended answer:** A+b — cookie sessions, admin-provisioned.

**User answer:** A + b, with the condition that migrating to B (bearer tokens) + a (self-registration) later must not require big rewrites.

**Status:** resolved

**Notes:** Design constraint: token table with kind (session/bearer), user creation as a service-layer function so self-registration is one route later, argon2id hashing.

### 33. Do per-user accounts pull conversation history into v1 scope?

**Recommended answer:** A — history in v1: per-user persisted conversations, sidebar list, resume, delete.

**User answer:** _(not recorded)_

**Status:** open

**Notes:** Revises Q7 where history was cut; accounts make it feel mandatory.

### 34. Do per-user accounts pull conversation history into v1?

**Recommended answer:** A — history in v1.

**User answer:** A — per-user persisted conversations with sidebar, resume, delete.

**Status:** resolved

### 35. Per-user settings in v1: per-user model choice from a configured model list, or no settings at all?

**Recommended answer:** A — per-user model choice from [[models]] config list, settings page, stored per user.

**User answer:** _(not recorded)_

**Status:** open

### 36. Per-user settings in v1?

**Recommended answer:** A — per-user model choice.

**User answer:** B — no per-user settings, no model choice. One global model from config.

**Status:** resolved

### 37. How are accounts created: CLI subcommand, web admin page, or both? And how is the first admin seeded?

**Recommended answer:** A — CLI create-user + config-seeded first admin; web admin later.

**User answer:** _(not recorded)_

**Status:** open

### 38. How are accounts created and how is the first admin seeded?

**Recommended answer:** A + config-seeded admin.

**User answer:** A + config seed — CLI create-user, first admin from TOML.

**Status:** resolved

### 39. What OS/management does the deployment host 192.168.68.128 run, and how much Nix plumbing should 42 ship (flake with nixos module vs plain binary)?

**Recommended answer:** Flake with package + nixos module/systemd unit outputs regardless, but the host's current state determines the path.

**User answer:** _(not recorded)_

**Status:** open

### 40. What OS/management does the deployment host run?

**Recommended answer:** Flake with package + nixos module outputs.

**User answer:** 192.168.68.128 is a NixOS box (26.11, nix 2.34.8), managed via /etc/nixos with a flake. SSH access available.

**Status:** resolved

**Notes:** So 42 ships as a flake with package + nixosModules output, enabled in that box's config.

### 41. Testing strategy: unit tests for pure logic + integration tests with mock LLM/SearXNG servers, no browser e2e?

**Recommended answer:** Yes — unit (citation validation, windowing, prompt builder, config) + integration (mock servers, temp SQLite), no e2e in v1.

**User answer:** _(not recorded)_

**Status:** open

### 42. Testing strategy?

**Recommended answer:** Unit + integration with mocks, no e2e.

**User answer:** Agreed on unit + integration, plus a Playwright e2e setup is wanted.

**Status:** resolved

**Notes:** E2E: small Playwright suite; browsers via nixpkgs chromium. Runner language (Node vs Rust bindings) is an implementation detail to settle at build time.

### 43. Repo layout: single crate with lib+bin split, flake.nix with package+nixosModules, e2e/ dir?

**Recommended answer:** Yes, as proposed.

**User answer:** _(not recorded)_

**Status:** open

### 44. Repo layout: single crate lib+bin, flake with package+nixosModules, e2e/ dir?

**Recommended answer:** As proposed.

**User answer:** Looks good — approved as proposed.

**Status:** resolved

## Agreed Decisions

- Local LLM only: any OpenAI-compatible HTTP endpoint (base_url + model in config); no model management in 42. Cloud providers possible later via the same interface.
- Search: existing SearXNG instance via JSON API (default <http://192.168.68.128:8888>), configured by URL. Verified working.
- Interface: web UI (vanilla HTML/CSS/JS, no build step, embedded via include_str!) plus a clean HTTP API on the same service.
- Implementation: Rust. Stack: axum + tokio + reqwest + rusqlite (spawn_blocking) + serde + tower-http + tracing.
- v1 features: multi-turn conversation, inline citations, SSE streaming, per-user accounts, per-user conversation history (sidebar, resume, delete). No per-user settings, no model picker, no search-only mode in v1.
- Citations: prompt-driven [n] markers, top 5-8 SearXNG results injected (snippets ~500 chars), backend validates/repairs markers against the source list.
- Multi-turn: re-search every turn; history bounded by sliding window (default last 10 turns, configurable) plus a configurable context_window (default 32768) that truncates oldest turns first.
- Streaming: SSE. POST /v1/ask {conversation_id, question} -> events: sources, token, done. Static UI at /.
- Conversation store: SQLite from day one, behind a ConversationStore trait. Path in TOML config.
- Auth: per-user accounts, first-class. argon2id password hashing, HTTP-only cookie sessions. Admin-provisioned (CLI: 42 admin create-user). First admin seeded from TOML config. Token table has a kind column (session/bearer) and user creation lives in the service layer, so bearer tokens and self-registration are additive later, not rewrites.
- Deployment: NixOS box at 192.168.68.128 (26.11). Flake with package + nixosModules outputs; enabled in that box's /etc/nixos config. TOML config file on disk.
- Testing: unit tests (citation validation, windowing, prompt builder, config), integration tests with in-process mock LLM/SearXNG servers and temp SQLite, plus a Playwright e2e suite (browsers via nixpkgs chromium).
- Repo layout: single crate (lib for engine/auth/store, bin for server + CLI subcommands serve/admin), src/{server,engine,auth,store,web}, e2e/ dir, flake.nix.

## Open Risks

- LLM endpoint details assumed from pi config: <http://192.168.68.128:9292/v1>, model Qwen3.8-27b-UD-Q6_K_M, 32k context. Confirm the llama.cpp server stays up and the model name is stable.
- Weak local models may produce poor citation discipline; the validator mitigates but answer quality depends on the model.
- Playwright runner language (Node vs Rust bindings) not yet decided; settle at build time.
- SearXNG on the .128 box is not a systemd unit named 'searxng'; how it runs there should be checked before finalizing deployment docs.
- Port: 4242 (locked in).
- UI theme/visual design not specified; assume minimal dark theme, refine during build.
- Self-registration and bearer tokens explicitly deferred; keep the token table and service-layer user creation upgrade-safe.

## Locked

- Service port: 4242.
- Default LLM: Qwen3.8-27b-UD-Q6_K_M at <http://192.168.68.128:9292/v1>.
- Dev environment: cachix devenv (devenv.nix + devenv.lock).
