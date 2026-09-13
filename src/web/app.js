// 42 frontend — vanilla JS, no build step.
// Views: login, chat (streaming answer + sources panel), conversation sidebar.
// API: POST /v1/auth/login, POST /v1/auth/logout,
//      POST /v1/ask (SSE: sources, token, done),
//      GET /v1/conversations, GET /v1/conversations/:id, DELETE /v1/conversations/:id

"use strict";

const app = document.getElementById("app");
app.textContent = "42 — scaffolding in progress";
