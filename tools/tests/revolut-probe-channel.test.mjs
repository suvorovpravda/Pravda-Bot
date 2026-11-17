// Regression: the debug-only concurrent probe reports its deny outcomes through
// the WebView document title, and the Rust title callback silently DROPS any
// payload longer than its bound. Adding IPC commands to the probe must never
// push the report past that bound: the native E2E would then poll an empty
// evidence log and every deny verdict would lose its proof.
// Run: node --test tools/tests/revolut-probe-channel.test.mjs
// The real probe script and the real E2E reader are executed; only the DOM, the
// Tauri bridge and the timer are stubbed. No WebView2 and no Windows needed.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const PROBE = new URL(
  "../../crates/pravda-app/frontend/security-probe-concurrent.js", import.meta.url);
const E2E = new URL("../test-ipc-security-windows.mjs", import.meta.url);
const MAIN_RS = new URL("../../crates/pravda-app/src/main.rs", import.meta.url);
const PREFIX = "PRAVDA_E2E_PROBE:";

/// The bound actually compiled into the desktop binary, not a copy of it.
function rustTitleBound() {
  const match = readFileSync(MAIN_RS, "utf8").match(/if payload\.len\(\) > ([0-9_]+)/);
  assert.ok(match, "the Rust title-channel bound must stay locatable in main.rs");
  return Number(match[1].replace(/_/g, ""));
}

/// The expansion the native E2E applies to every decoded row, read from the
/// E2E source so this test cannot pass against a stale duplicate.
function e2eExpander() {
  const source = readFileSync(E2E, "utf8");
  const start = source.indexOf("function expandProbeResult(");
  assert.ok(start >= 0, "the native E2E must expand the compact probe rows");
  const end = source.indexOf("\n}", start);
  assert.ok(end > start, "expandProbeResult must be a complete function");
  const sandbox = {};
  runInNewContext(source.slice(start, end + 2) +
    "\nglobalThis.expand = expandProbeResult;\n", sandbox);
  return sandbox.expand;
}

/// Execute the real probe against a stubbed bridge that always denies.
async function probeTitle(denialMessage, bridge = true) {
  const source = readFileSync(PROBE, "utf8") + "\nglobalThis.__ready = check();\n";
  const document = { title: "" };
  const sandbox = {
    document,
    console,
    btoa: value => Buffer.from(value, "binary").toString("base64"),
    setInterval: () => 0,
    window: bridge
      ? { __TAURI__: { core: { invoke: async () => { throw denialMessage; } } } }
      : {}
  };
  runInNewContext(source, sandbox);
  await sandbox.__ready;
  assert.ok(document.title.startsWith(PREFIX), "the probe must emit its title proof");
  return document.title.slice(PREFIX.length);
}

function decode(payload) {
  return JSON.parse(Buffer.from(payload, "base64").toString("utf8"));
}

/// The command list the native E2E requires evidence for, read from its source.
function e2eExpectedCommands() {
  const source = readFileSync(E2E, "utf8");
  const start = source.indexOf("async function checkConcurrentWindowAfterAuth");
  assert.ok(start >= 0, "the concurrent deny check must stay locatable");
  const list = source.indexOf("for (const command of [", start);
  const close = source.indexOf("])", list);
  assert.ok(list > start && close > list, "the expected command list must be readable");
  return [...source.slice(list, close).matchAll(/"([^"]+)"/g)].map(match => match[1]);
}

test("the whole probe report stays inside the Rust title-channel bound", async () => {
  const bound = rustTitleBound();
  const acl = await probeTitle("Command ipc_revolut_private_status not allowed by ACL");
  // The longest classification the probe can emit, so the assertion holds for
  // every denial layer and not only for the happy ACL path.
  const longest = await probeTitle("FORBIDDEN_WINDOW");
  const worst = Math.max(acl.length, longest.length);
  assert.ok(worst + 16 <= bound,
    `probe payload ${worst} base64 chars (+16 for sequence growth) must fit the ` +
    `Rust bound ${bound}; an oversized title is dropped and no deny verdict has evidence`);
  assert.ok(decode(acl).results.length > 0, "the report must carry results");
});

test("every command the native E2E asks for is carried and expanded losslessly", async () => {
  const expand = e2eExpander();
  const proof = decode(await probeTitle("Command not allowed by capability"));
  const rows = proof.results.map(expand);
  const expected = e2eExpectedCommands();
  assert.ok(expected.length > 0, "the E2E must require at least one command");
  for (const command of expected) {
    const row = rows.find(item => item.command === command);
    assert.ok(row, `missing probe result for ${command}`);
    assert.equal(row.state, "rejected", command);
    assert.equal(row.access_denied, true, command);
    assert.equal(row.denial_layer, "tauri_acl", command);
    assert.equal(row.malformed, false, command);
  }
});

test("a missing Tauri bridge is never expanded into an ACL denial", async () => {
  const expand = e2eExpander();
  const proof = decode(await probeTitle("unused", false));
  const rows = proof.results.map(expand);
  assert.ok(rows.length > 0, "the probe must still report without a bridge");
  for (const row of rows) {
    assert.equal(row.state, "no-bridge", row.command);
    assert.notEqual(row.access_denied, true, row.command);
    assert.notEqual(row.denial_layer, "tauri_acl", row.command);
  }
});
