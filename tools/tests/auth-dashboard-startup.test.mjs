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

async function boot({ready, logoutForm, revokeBeforeLoad = false}) {
  const shell = element(), body = element();
  const refreshCalls = [];
  const refreshHooks = [
    "refreshRevolutConsent", "refreshAll", "refreshSecurity",
    "refreshUpdateReadiness", "refreshSecurityEvents", "refreshPaper"
  ];
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
  // The shell must not flash into view while dashboard.js is still loading.
  assert.equal(shell.hidden, true, "shell must remain hidden before dashboard readiness");
  assert.equal(screen.hidden, false, "auth screen must remain visible during loading");
  // Simulate dashboard.js installing its refresh hooks before its load event.
  // Every hook must run only after the Rust-authenticated shell is revealed.
  for (const name of refreshHooks) {
    window[name] = () => refreshCalls.push({
      name,
      shellHidden: shell.hidden,
      screenHidden: screen.hidden
    });
  }
  window.pravdaDashboardReady = ready;
  if (revokeBeforeLoad) window.pravdaRequireLogin();
  script.listeners.get("load")();
  return { shell, screen, card, logoutForm, window, refreshCalls };
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

test("stale dashboard load cannot reopen shell after session revocation", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm, revokeBeforeLoad: true });
  assert.equal(state.shell.hidden, true);
  assert.equal(state.screen.hidden, false);
  assert.equal(logoutForm.listeners.has("submit"), false);
});

test("first authenticated dashboard refreshes gated IPC modules after reveal", async () => {
  const state = await boot({ ready: true, logoutForm: element() });
  assert.deepEqual(state.refreshCalls.map(call => call.name), [
    "refreshRevolutConsent", "refreshAll", "refreshSecurity",
    "refreshUpdateReadiness", "refreshSecurityEvents", "refreshPaper"
  ]);
  assert.ok(state.refreshCalls.every(call => !call.shellHidden && call.screenHidden),
    "no protected refresh may run while the authenticated shell is hidden");
});

test("missing logout control never triggers authenticated dashboard refresh", async () => {
  const state = await boot({ ready: true, logoutForm: null });
  assert.equal(state.refreshCalls.length, 0);
});

test("revoked login cannot trigger delayed dashboard refresh", async () => {
  const state = await boot({ ready: true, logoutForm: element(), revokeBeforeLoad: true });
  assert.equal(state.refreshCalls.length, 0);
});
