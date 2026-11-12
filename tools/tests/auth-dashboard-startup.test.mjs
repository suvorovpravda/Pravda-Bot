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

async function boot({ready, logoutForm, revokeBeforeLoad = false, repeatLoginBeforeLoad = false, revokeAndReloginBeforeLoad = false, reloginAfterLoad = false, logoutRejects = false}) {
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
    __TAURI__: { core: { invoke: async command => { if (command === "ipc_auth_logout" && logoutRejects) throw new Error("IPC_DOWN"); return command === "ipc_auth_status"; } } },
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
  if (repeatLoginBeforeLoad || revokeAndReloginBeforeLoad) {
    if (revokeAndReloginBeforeLoad) window.pravdaRequireLogin();
    // A successful Rust login replaces the token and increments the JS
    // authentication generation without injecting a second dashboard script.
    await loginSubmit({ preventDefault() {} });
    assert.equal(body.children.filter(child => child.src === "./dashboard.js").length, 1,
      "concurrent login confirmations must reuse the single pending script");
  } else if (revokeBeforeLoad) {
    window.pravdaRequireLogin();
  }
  script.listeners.get("load")();
  if (reloginAfterLoad) {
    await loginSubmit({ preventDefault() {} });
    assert.equal(body.children.filter(child => child.src === "./dashboard.js").length, 1,
      "a loaded script must never be reinjected after session restoration");
  }
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

test("stale dashboard load stays hidden but records successful initialization for next login", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm, revokeBeforeLoad: true });
  assert.equal(state.shell.hidden, true);
  assert.equal(state.screen.hidden, false);
  assert.equal(logoutForm.listeners.has("submit"), true,
    "initialized logout control may bind while hidden; it grants no session");
  assert.equal(state.refreshCalls.length, 0);
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

test("two successful logins before script load complete one dashboard without duplicate binding", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm, repeatLoginBeforeLoad: true });
  assert.equal(state.shell.hidden, false);
  assert.equal(state.screen.hidden, true);
  assert.equal(logoutForm.listeners.has("submit"), true);
  assert.equal(state.refreshCalls.length, 6,
    "the single pending dashboard load refreshes latest authenticated session once");
  assert.ok(state.refreshCalls.every(call => !call.shellHidden && call.screenHidden));
});

test("logout followed by fresh login while script loads permits only the new session", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm, revokeAndReloginBeforeLoad: true });
  assert.equal(state.shell.hidden, false);
  assert.equal(state.screen.hidden, true);
  assert.equal(logoutForm.listeners.has("submit"), true);
  assert.equal(state.refreshCalls.length, 6);
});

test("two successful logins cannot bypass a missing logout control", async () => {
  const state = await boot({ ready: true, logoutForm: null, repeatLoginBeforeLoad: true });
  assert.equal(state.shell.hidden, true);
  assert.equal(state.screen.hidden, false);
  assert.equal(state.refreshCalls.length, 0);
});

test("relogin after a revoked script load reuses initialized JS without second evaluation", async () => {
  const logoutForm = element();
  const state = await boot({
    ready: true, logoutForm, revokeBeforeLoad: true, reloginAfterLoad: true
  });
  assert.equal(state.shell.hidden, false);
  assert.equal(state.screen.hidden, true);
  assert.equal(logoutForm.listeners.has("submit"), true);
  assert.equal(state.refreshCalls.length, 6);
});

test("failed readiness after revocation does not expose shell or trust the failed script", async () => {
  const state = await boot({ ready: false, logoutForm: element(), revokeBeforeLoad: true });
  assert.equal(state.shell.hidden, true);
  assert.equal(state.screen.hidden, false);
  assert.equal(state.refreshCalls.length, 0);
});

test("failed Rust logout never pretends the session was revoked", async () => {
  const logoutForm = element();
  const state = await boot({ ready: true, logoutForm, logoutRejects: true });
  assert.equal(state.shell.hidden, false);
  await logoutForm.listeners.get("submit")({ preventDefault() {} });
  assert.equal(state.shell.hidden, true, "unconfirmed logout must hide protected dashboard");
  assert.equal(state.screen.hidden, false, "show a blocking recovery screen");
  assert.equal(state.card.children.some(child => child.className === "ipc-startup-recovery"), true);
  assert.equal(state.window.pravdaAuthGeneration, 1,
    "do not mark a Rust session revoked when logout IPC failed");
});
