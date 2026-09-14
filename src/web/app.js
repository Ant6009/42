// 42 frontend — vanilla JS, no build step, no external dependencies.
// Views: login, chat (streaming answer + sources panel), conversation sidebar.
// API: POST /v1/auth/login, POST /v1/auth/logout, GET /v1/me,
//      POST /v1/ask (SSE: conversation, sources, token, final, done, error),
//      GET/POST /v1/conversations, GET/DELETE /v1/conversations/:id

"use strict";

const state = {
  user: null,
  isAdmin: false,
  conversations: [],
  currentId: null,
  streaming: false,
  sources: [],
};

const $ = (id) => document.getElementById(id);

/* ------------------------------------------------------------------ api */

async function api(path, options = {}) {
  const res = await fetch(path, {
    credentials: "same-origin",
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (res.status === 401) {
    showLogin();
    throw new Error("unauthorized");
  }
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res.json();
}

/* -------------------------------------------------------------- views */

function showLogin() {
  state.user = null;
  $("login-view").hidden = false;
  $("chat-view").hidden = true;
  $("conversation-list").innerHTML = "";
  $("logout-btn").hidden = true;
}

function showChat() {
  $("login-view").hidden = true;
  $("settings-view").hidden = true;
  $("chat-view").hidden = false;
  $("logout-btn").hidden = false;
  $("settings-btn").hidden = !state.isAdmin;
  $("user-label").textContent = state.user.username;
  renderConversationList();
}

function showSettings() {
  $("chat-view").hidden = true;
  $("settings-view").hidden = false;
}

function hideSettings() {
  $("settings-view").hidden = true;
  $("chat-view").hidden = false;
}

/* ---------------------------------------------------- conversations */

function renderConversationList() {
  const list = $("conversation-list");
  list.innerHTML = "";
  for (const conv of state.conversations) {
    const item = document.createElement("div");
    item.className = "conv-item" + (conv.id === state.currentId ? " active" : "");
    const title = document.createElement("span");
    title.className = "title";
    title.textContent = conv.title || "New conversation";
    const del = document.createElement("button");
    del.className = "del";
    del.textContent = "✕";
    del.title = "Delete";
    del.onclick = (e) => {
      e.stopPropagation();
      deleteConversation(conv.id);
    };
    item.appendChild(title);
    item.appendChild(del);
    item.onclick = () => openConversation(conv.id);
    list.appendChild(item);
  }
}

async function refreshConversations() {
  state.conversations = await api("/v1/conversations");
  renderConversationList();
}

function newChat() {
  state.currentId = null;
  state.sources = [];
  $("messages").innerHTML = "";
  renderConversationList();
  closeSidebarIfMobile();
  $("question").focus();
}

async function openConversation(id) {
  if (state.streaming) return;
  state.currentId = id;
  state.sources = [];
  const data = await api(`/v1/conversations/${id}`);
  const messages = $("messages");
  messages.innerHTML = "";
  for (const m of data.messages) {
    appendMessage(m.role, m.content, m.sources || []);
  }
  renderConversationList();
  closeSidebarIfMobile();
  scrollToBottom();
}

async function deleteConversation(id) {
  await fetch(`/v1/conversations/${id}`, { method: "DELETE", credentials: "same-origin" });
  state.conversations = state.conversations.filter((c) => c.id !== id);
  if (state.currentId === id) newChat();
  else renderConversationList();
}

/* ---------------------------------------------------------------- chat */

function appendMessage(role, content, sources) {
  const msg = document.createElement("div");
  msg.className = `msg ${role}`;
  const label = document.createElement("div");
  label.className = "role";
  label.textContent = role === "user" ? "You" : "42";
  const body = document.createElement("div");
  if (role === "user") {
    body.className = "bubble";
    body.textContent = content;
  } else {
    body.className = "answer";
    setSafeHtml(body, renderMarkdown(content, sources || []));
    if (sources && sources.length) body.appendChild(renderSources(sources));
  }
  msg.appendChild(label);
  msg.appendChild(body);
  $("messages").appendChild(msg);
  return body;
}

function renderSources(sources) {
  const wrap = document.createElement("div");
  wrap.className = "sources";
  const title = document.createElement("div");
  title.className = "sources-title";
  title.textContent = `Sources (${sources.length})`;
  wrap.appendChild(title);
  for (const s of sources) {
    const a = document.createElement("a");
    a.className = "source-item";
    a.href = s.url;
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    a.dataset.srcIndex = s.index;
    const num = document.createElement("span");
    num.className = "num";
    num.textContent = `[${s.index}]`;
    const t = document.createElement("span");
    t.textContent = s.title;
    const url = document.createElement("span");
    url.className = "url";
    url.textContent = s.url;
    a.append(num, t, url);
    wrap.appendChild(a);
  }
  return wrap;
}

function flashSource(index) {
  const el = document.querySelector(`.source-item[data-src-index="${index}"]`);
  if (!el) return;
  el.classList.add("flash");
  el.scrollIntoView({ block: "nearest", behavior: "smooth" });
  setTimeout(() => el.classList.remove("flash"), 1200);
}

async function ask() {
  const question = $("question").value.trim();
  if (!question || state.streaming) return;
  state.streaming = true;
  $("ask-btn").disabled = true;
  $("question").value = "";

  appendMessage("user", question);
  const answerBody = appendMessage("assistant", "");
  answerBody.classList.add("typing");
  let answer = "";
  let sources = [];

  try {
    const res = await fetch("/v1/ask", {
      method: "POST",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        conversation_id: state.currentId,
        question,
      }),
    });
    if (res.status === 401) {
      showLogin();
      return;
    }
    if (!res.ok) throw new Error(`HTTP ${res.status}`);

    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buf = "";
    let gotError = null;
    let thinkingShown = false;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      let nl;
      while ((nl = buf.indexOf("\n")) !== -1) {
        const line = buf.slice(0, nl).trim();
        buf = buf.slice(nl + 1);
        if (!line.startsWith("data:")) continue;
        let evt;
        try {
          evt = JSON.parse(line.slice(5).trim());
        } catch {
          continue;
        }
        switch (evt.type) {
          case "conversation":
            state.currentId = evt.id;
            break;
          case "sources":
            sources = evt.sources || [];
            state.sources = sources;
            break;
          case "thinking":
            // Reasoning phase: show an indicator until the answer starts.
            if (!answer && !thinkingShown) {
              const s = document.createElement("span");
              s.className = "muted";
              s.textContent = "Thinking\u2026";
              answerBody.replaceChildren(s);
              thinkingShown = true;
            }
            break;
          case "token":
            answer += evt.text;
            setSafeHtml(answerBody, renderMarkdown(answer, sources));
            if (sources.length) answerBody.appendChild(renderSources(sources));
            scrollToBottom();
            break;
          case "final":
            answer = evt.text;
            setSafeHtml(answerBody, renderMarkdown(answer, sources));
            if (sources.length) answerBody.appendChild(renderSources(sources));
            break;
          case "error":
            gotError = evt.message;
            break;
        }
      }
    }

    if (gotError) {
      answerBody.replaceChildren();
      const p = document.createElement("p");
      p.className = "error";
      p.textContent = `Error: ${gotError}`;
      answerBody.appendChild(p);
    } else if (answer) {
      // Re-render once with citation links wired to the sources panel.
      setSafeHtml(answerBody, renderMarkdown(answer, sources));
      if (sources.length) answerBody.appendChild(renderSources(sources));
    } else {
      const p = document.createElement("p");
      p.className = "muted";
      p.textContent = "No answer received.";
      answerBody.replaceChildren(p);
    }
  } catch (e) {
    answerBody.replaceChildren();
    const p = document.createElement("p");
    p.className = "error";
    p.textContent = `Error: ${e.message}`;
    answerBody.appendChild(p);
  } finally {
    answerBody.classList.remove("typing");
    state.streaming = false;
    $("ask-btn").disabled = false;
    scrollToBottom();
    refreshConversations().catch(() => {});
  }
}

function scrollToBottom() {
  const m = $("messages");
  m.scrollTop = m.scrollHeight;
}

/* ------------------------------------------------------ sanitizing */

// renderMarkdown escapes all input before adding tags, so its output is
// safe by construction. As defense in depth, every rendered fragment is
// still pushed through this whitelist sanitizer before entering the DOM:
// unknown tags are dropped, unknown attributes stripped, and hrefs
// restricted to http(s). A bug in the markdown layer cannot inject markup.
const ALLOWED_TAGS = new Set([
  "p", "br", "strong", "em", "code", "pre",
  "h1", "h2", "h3", "ul", "ol", "li", "a", "blockquote", "sup",
]);
const ALLOWED_ATTRS = new Set(["href", "target", "rel", "class", "data-cite", "data-src-index"]);

function sanitizeNode(parent) {
  for (const child of [...parent.childNodes]) {
    if (child.nodeType === Node.TEXT_NODE) continue;
    if (child.nodeType !== Node.ELEMENT_NODE) {
      parent.removeChild(child);
      continue;
    }
    const tag = child.tagName.toLowerCase();
    if (!ALLOWED_TAGS.has(tag)) {
      parent.removeChild(child);
      continue;
    }
    for (const attr of [...child.attributes]) {
      if (!ALLOWED_ATTRS.has(attr.name.toLowerCase())) child.removeAttribute(attr.name);
    }
    if (tag === "a") {
      const href = child.getAttribute("href") || "";
      if (!/^https?:\/\//.test(href)) child.removeAttribute("href");
    }
    sanitizeNode(child);
  }
}

function setSafeHtml(el, html) {
  // Parse in a separate inert document, scrub it, then move nodes over.
  const doc = new DOMParser().parseFromString(html, "text/html");
  sanitizeNode(doc.body);
  el.replaceChildren();
  for (const child of [...doc.body.childNodes]) el.append(child);
}

/* -------------------------------------------------------- markdown */

// Minimal, dependency-free markdown: escapes HTML first, so user/LLM
// content cannot inject markup. Supports: code fences, inline code,
// bold, italic, links, headings, lists, blockquotes, paragraphs, and
// [n] citation markers.
function renderMarkdown(text, sources = []) {
  const srcSet = new Set(sources.map((s) => s.index));
  const esc = (s) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  // Extract fenced code blocks first so their content is untouched.
  const codeBlocks = [];
  text = text.replace(/```(\w*)\n([\s\S]*?)```/g, (_m, _lang, code) => {
    codeBlocks.push(`<pre><code>${esc(code.replace(/\n$/, ""))}</code></pre>`);
    return `\u0000CODE${codeBlocks.length - 1}\u0000`;
  });

  let html = esc(text);

  html = html.replace(/`([^`\n]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/(^|\W)\*([^*\n]+)\*(?=\W|$)/g, "$1<em>$2</em>");
  html = html.replace(
    /\[([^\]\n]+)\]\((https?:[^)\s]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
  );
  // Citation markers [1] / [2,3] -> clickable sup links.
  html = html.replace(/\[(\d+(?:,\d+)*)\]/g, (_, nums) => {
    const parts = nums.split(",").filter((n) => srcSet.has(Number(n)));
    if (!parts.length) return `[${nums}]`;
    return parts
      .map((n) => `<sup><a class="cite" data-cite="${n}">[${n}]</a></sup>`)
      .join("");
  });
  html = html.replace(/^### (.*)$/gm, "<h3>$1</h3>");
  html = html.replace(/^## (.*)$/gm, "<h2>$1</h2>");
  html = html.replace(/^# (.*)$/gm, "<h1>$1</h1>");
  html = html.replace(/^&gt; (.*)$/gm, "<blockquote>$1</blockquote>");
  html = html.replace(/^[-*] (.*)$/gm, "<li>$1</li>");
  html = html.replace(/(<li>[\s\S]*?<\/li>)(?!\s*<li>)/g, "<ul>$1</ul>");

  // Paragraphs: split on blank lines, wrap non-block lines.
  html = html
    .split(/\n{2,}/)
    .map((block) => {
      const b = block.trim();
      if (!b) return "";
      if (/^<(h\d|ul|ol|pre|blockquote|p)/.test(b) || b.includes("\u0000CODE")) return b;
      return `<p>${b.replace(/\n/g, "<br>")}</p>`;
    })
    .join("");

  html = html.replace(/\u0000CODE(\d+)\u0000/g, (_, i) => codeBlocks[Number(i)]);
  return html;
}

/* -------------------------------------------------------------- boot */

async function boot() {
  try {
    state.user = await api("/v1/me");
    state.isAdmin = !!state.user.is_admin;
    showChat();
    await refreshConversations();
  } catch {
    showLogin();
  }
}

$("login-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const err = $("login-error");
  err.hidden = true;
  try {
    state.user = await api("/v1/auth/login", {
      method: "POST",
      body: JSON.stringify({
        username: $("login-username").value,
        password: $("login-password").value,
      }),
    });
    $("login-password").value = "";
    showChat();
    await refreshConversations();
  } catch (ex) {
    err.textContent = ex.message === "unauthorized" ? "Invalid credentials" : ex.message;
    err.hidden = false;
  }
});

$("logout-btn").addEventListener("click", async () => {
  await fetch("/v1/auth/logout", { method: "POST", credentials: "same-origin" });
  showLogin();
});

/* ---------------------------------------------------------- settings */

const SETTING_FIELDS = [
  ["set-llm-base-url", "llm_base_url"],
  ["set-llm-model", "llm_model"],
  ["set-llm-context-window", "llm_context_window"],
  ["set-search-url", "search_url"],
  ["set-search-max-results", "search_max_results"],
  ["set-search-snippet-chars", "search_snippet_chars"],
  ["set-history-window", "history_window"],
];

async function openSettings() {
  if (!state.isAdmin) return;
  try {
    const s = await api("/v1/settings");
    for (const [id, key] of SETTING_FIELDS) $(id).value = s[key];
    $("settings-status").textContent = "";
    showSettings();
  } catch (e) {
    alert(`Could not load settings: ${e.message}`);
  }
}

$("settings-btn").addEventListener("click", openSettings);
$("settings-back-btn").addEventListener("click", hideSettings);
$("settings-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const status = $("settings-status");
  const body = {};
  for (const [id, key] of SETTING_FIELDS) {
    const raw = $(id).value.trim();
    body[key] = /^[0-9]+$/.test(raw) ? Number(raw) : raw;
  }
  try {
    await api("/v1/settings", { method: "PUT", body: JSON.stringify(body) });
    status.textContent = "Saved";
    setTimeout(() => (status.textContent = ""), 2000);
  } catch (ex) {
    status.textContent = ex.message;
  }
});

/* ---------------------------------------------------- mobile sidebar */

function isMobile() {
  return window.matchMedia("(max-width: 768px)").matches;
}

function setSidebar(open) {
  $("sidebar").classList.toggle("open", open);
  $("sidebar-backdrop").hidden = !open;
}

function closeSidebarIfMobile() {
  if (isMobile()) setSidebar(false);
}

$("menu-btn").addEventListener("click", () => setSidebar(true));
$("sidebar-backdrop").addEventListener("click", () => setSidebar(false));
window.addEventListener("resize", () => {
  if (!isMobile()) setSidebar(false);
});

$("new-chat-btn").addEventListener("click", newChat);
$("ask-form").addEventListener("submit", (e) => {
  e.preventDefault();
  ask();
});

$("question").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    ask();
  }
});

// Citation clicks: highlight the matching source in the panel.
document.addEventListener("click", (e) => {
  const cite = e.target.closest(".cite");
  if (cite) {
    e.preventDefault();
    flashSource(Number(cite.dataset.cite));
  }
});

boot();
