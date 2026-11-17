// Regression: the private Revolut X panel must not render a reply that was
// authorized before a consent revocation, and a venue credential refusal must
// never be mistaken for a local session expiry.
// Run: node --test tools/tests/revolut-private-ui.test.mjs
// The real dashboard.js section is executed; only the DOM and IPC are stubbed.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const source = readFileSync(
  new URL("../../crates/pravda-app/frontend/dashboard.js", import.meta.url), "utf8");
const start = source.indexOf("    let revolutPrivateOptIn = false;");
const end = source.indexOf('    $("revolut-private-setup").addEventListener');
assert.ok(start >= 0 && end > start, "the private connection section must be locatable");
const section = source.slice(start, end) +
  "\nwindow.__probe = {refreshRevolutPrivate, setRevolutPrivateConsent, readRevolutPrivateStatus,\n  openRevolutPrivateAssistant};\n";

const STATUS_ENABLED = ["connection|configured", "consent|enabled", "scope|read_only",
  "orders|disabled", "vault|windows_credential_manager"];
const STATUS_DISABLED = ["connection|configured", "consent|disabled", "scope|read_only",
  "orders|disabled", "vault|windows_credential_manager"];
const BALANCES = ["status|ok", "observed_at|1763000000000", "count|1",
  "balance|EUR|10.5|0|10.5|none"];

function element() {
  return {
    textContent: "", className: "", checked: false, disabled: false, children: [],
    append(...items) { this.children.push(...items); },
    replaceChildren() { this.children = []; this.textContent = ""; }
  };
}

function boot(handlers) {
  const nodes = new Map();
  const $ = id => {
    if (!nodes.has(id)) nodes.set(id, element());
    return nodes.get(id);
  };
  const calls = [];
  const window = {
    pravdaAuthGeneration: 1,
    pravdaRequireLogin() { calls.push("requireLogin"); },
    pravdaInvoke(command, args) {
      calls.push(command);
      return Promise.resolve().then(() => handlers[command](args));
    }
  };
  const document = { createElement: () => element() };
  runInNewContext(section, {
    window, document, console, $, securitySessionReady: () => true
  });
  return { window, nodes, $, calls, probe: window.__probe };
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

const settle = () => new Promise(resolve => setImmediate(resolve));

// P1: a signed reply that was authorized before the revocation must be dropped.
test("private balances arriving after consent revocation are never rendered", async () => {
  const balances = deferred();
  // The stub mirrors Rust: the persisted consent is what the next status read reports.
  let consent = true;
  const harness = boot({
    ipc_revolut_private_status: () => consent ? STATUS_ENABLED : STATUS_DISABLED,
    ipc_revolut_private_consent_set: ({ enabled }) => { consent = enabled; return enabled; },
    ipc_revolut_private_balances: () => balances.promise
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  assert.equal(harness.$("revolut-private-test").disabled, false, "consent enabled arm must be ready");

  const reading = harness.probe.refreshRevolutPrivate();
  await settle();
  // The operator revokes while the signed GET is still in flight.
  const revoking = harness.probe.setRevolutPrivateConsent(false);
  await settle();
  balances.resolve(BALANCES);
  await Promise.all([reading, revoking]);
  await settle();
  await settle();

  const shown = harness.$("revolut-private-balances");
  assert.equal(shown.children.length, 0, "no balance table may be rendered after revocation");
  assert.ok(!String(shown.textContent).includes("EUR"), "no currency may survive a revocation");
  assert.ok(!String(harness.$("revolut-private-checked").textContent).includes("Lecture authentifiée à"),
    "the success banner must not be written after revocation");
  assert.equal(harness.$("revolut-private-test").disabled, true, "the read action stays disarmed");
});

// P2: Revolut X refusing the API key is not a local session expiry.
test("PRIVATE_VENUE_UNAUTHORIZED never locks the local session", async () => {
  const harness = boot({
    ipc_revolut_private_status: () => STATUS_ENABLED,
    ipc_revolut_private_balances: () => { throw "PRIVATE_VENUE_UNAUTHORIZED"; }
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  await harness.probe.refreshRevolutPrivate();
  await settle();
  assert.equal(harness.calls.filter(c => c === "requireLogin").length, 0,
    "a venue credential refusal must not force a local re-login");
  assert.match(harness.$("revolut-private-checked").textContent, /Identifiants refusés par Revolut X/);
  assert.equal(harness.$("revolut-private-balances").textContent, "Aucun solde vérifié.");
});

// Non-regression: the closed local code must still lock the session.
test("the closed local UNAUTHORIZED code still requires a new login", async () => {
  const harness = boot({
    ipc_revolut_private_status: () => STATUS_ENABLED,
    ipc_revolut_private_balances: () => { throw "UNAUTHORIZED"; }
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  await harness.probe.refreshRevolutPrivate();
  await settle();
  assert.equal(harness.calls.filter(c => c === "requireLogin").length, 1,
    "a genuinely expired local session must still be relocked");
  assert.match(harness.$("revolut-private-checked").textContent, /Session locale expirée/);
});

// Non-regression: a reply from a previous login must never be rendered.
test("private balances arriving after a logout are never rendered", async () => {
  const balances = deferred();
  const harness = boot({
    ipc_revolut_private_status: () => STATUS_ENABLED,
    ipc_revolut_private_balances: () => balances.promise
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  const reading = harness.probe.refreshRevolutPrivate();
  await settle();
  harness.window.pravdaAuthGeneration = 2;
  balances.resolve(BALANCES);
  await reading;
  await settle();
  assert.equal(harness.$("revolut-private-balances").children.length, 0,
    "a stale session reply must not be rendered");
});

// The assistant must not lock the session for a platform or availability refusal.
test("assistant failures other than UNAUTHORIZED keep the local session", async () => {
  const harness = boot({
    ipc_revolut_private_status: () => STATUS_ENABLED,
    ipc_revolut_private_setup: () => { throw "PRIVATE_SETUP_UNAVAILABLE"; }
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  await harness.probe.openRevolutPrivateAssistant();
  await settle();
  assert.equal(harness.calls.filter(c => c === "requireLogin").length, 0,
    "an unavailable assistant must not force a local re-login");
  assert.match(harness.$("revolut-private-checked").textContent, /Assistant d’import indisponible/);
});

// P3: a revocation that Rust could not persist must be reported as non durable.
test("an unpersistable revocation warns that the stored opt-in must be removed", async () => {
  // Rust closes memory first, then fails to persist: the next status read must
  // report the memory state (disabled), never the stale file.
  let memory = true;
  const harness = boot({
    ipc_revolut_private_status: () => memory ? STATUS_ENABLED : STATUS_DISABLED,
    ipc_revolut_private_consent_set: () => { memory = false; throw "PRIVATE_CONSENT_REVOKE_UNSAFE"; }
  });
  await harness.probe.readRevolutPrivateStatus();
  await settle();
  await harness.probe.setRevolutPrivateConsent(false);
  await settle();
  const status = harness.$("revolut-private-consent-status").textContent;
  assert.match(status, /Révocation non durable/, "the operator must be warned");
  assert.match(status, /revolut_private_consent\.cfg/, "the file to remove must be named");
  assert.equal(harness.$("revolut-private-consent").checked, false, "the gate stays closed");
});
