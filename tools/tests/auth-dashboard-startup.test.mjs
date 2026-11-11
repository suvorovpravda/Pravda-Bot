// Regression: a dashboard script load is not proof that the logout form exists.
// Run: node --test tools/tests/auth-dashboard-startup.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const source = readFileSync(new URL("../../crates/pravda-app/frontend/auth.js", import.meta.url), "utf8");

function element() {
  const listeners = new Map();
  return {
    hidden: false, value: "", textContent: "", className: "",
    style: {}, required: false, disabled: false,
    listeners, children: [],
    addEventListener(name, callback) { listeners.set(name, callback); },
    append(...children) { this.children.push(...children); },
    setAttribute() {}, focus() {}, reset() {}, select() {},
    querySelector() { return null; }, remove() {}
  };
}

async function boot({ready, logoutForm}) {
  const shell = element(), body = element();
  const document = {
    body,
    querySelector(selector) { return selector === "main.shell" ? shell : null; },
    createElement() { return element(); },
    getElementById(id) { return id === "ipc-logout-form" ? logoutForm : null; }
  };
  const window = {
    __TAURI__: { core: { invoke: async command => command === "ipc_auth_status" } },
    addEventListener() {},
    pravdaDashboardReady: ready,
    location: { reload() {} }
  };
  // Initial auth status runs asynchronously and calls showLogin.
  runInNewContext(source, { window, document, console });
  await new Promise(resolve => setImmediate(resolve));
  const screen = body.children[0];
  const card = screen.children[0];
  const loginForm = card.children.find(child => child.listeners.has("submit"));
  assert.ok(loginForm, "login form must be present");
  const loginSubmit = loginForm.listeners.get("submit");
  await loginSubmit({ preventDefault() {} });
  const script = body.children.find(child => child.src === "./dashboard.js");
  assert.ok(script, "dashboard script must be inserted");
  window.pravdaDashboardReady = ready;
  script.listeners.get("load")();
  return { shell, screen, card, logoutForm, window };
}

test("missing logout form fails closed even if dashboard signals ready", async () => {
  const state = await boot({ ready: true, logoutForm: null });
  assert.equal(state.shell.hidden, true);
  assert.equal(state.screen.hidden, false);
  assert.equal(state.card.children.some(child => child.className === "ipc-startup-recovery"), true);
});

test("unready dashboard fails closed without binding logout", async () => {
  const logoutForm = element();
  const state = await boot({ ready: false, logoutForm });
  assert.equal(state.shell.hidden, true);
  assert.equal(logoutForm.listeners.has("submit"), false);
});

test("ready dashboard with logout form binds logout", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm });
  assert.equal(state.shell.hidden, false);
  assert.equal(logoutForm.listeners.has("submit"), true);
});
