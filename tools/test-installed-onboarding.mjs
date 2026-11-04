#!/usr/bin/env node
// P0 #106: real installed release WebView2 GUI acceptance (MSI and NSIS).
// No mocks, no dev binary, no HTTP application server, no trading orders.
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { randomBytes, createHash } from "node:crypto";

const exe = resolve(process.env.PRAVDA_INSTALLED_EXE || "");
const auth = resolve(process.env.PRAVDA_AUTH_PATH || "");
const db = resolve(process.env.PRAVDA_DB_PATH || "");
const kind = process.env.PRAVDA_INSTALL_KIND;
const reportPath = process.env.PRAVDA_ONBOARDING_REPORT;
const port = Number(process.env.PRAVDA_ONBOARDING_DRIVER_PORT || "4445");
const report = {
  version: 1, kind, sha: process.env.GITHUB_SHA || "",
  windows: process.env.PRAVDA_E2E_WINDOWS_VERSION || "",
  webview2: process.env.PRAVDA_E2E_WEBVIEW2_VERSION || "",
  scope: "Installed release GUI: setup, process restart, login, logout",
  limits: "WebView2 present on CI runner; does not prove air-gapped WebView2 installation",
  passed: false, tests: []
};
let child = null, session = null, failure = null;
const digest = path => createHash("sha256").update(readFileSync(path)).digest("hex");
function check(name, ok) {
  report.tests.push({name, passed: Boolean(ok)});
  console.log("[" + (ok ? "PASS" : "FAIL") + "] " + name);
  assert.ok(ok, name);
}
async function http(method, path, payload, timeout = 20000) {
  const response = await fetch("http://127.0.0.1:" + port + path, {
    method, headers: {"content-type": "application/json"},
    body: payload === undefined ? undefined : JSON.stringify(payload),
    signal: AbortSignal.timeout(timeout)
  });
  const data = await response.json();
  if (!response.ok ||
      (typeof data.value?.error === "string" && typeof data.value?.message === "string"))
    throw Error("WebDriver protocol rejected " + method + " " + path +
      " (" + response.status + "): " + String(data.value?.error || "").slice(0,80));
  return data.value;
}
const wd = (method, path, payload) =>
  http(method, "/session/" + session + path, payload);
async function until(predicate, label, limit = 80) {
  for (let i = 0; i < limit; i++) {
    if (await predicate()) return;
    await new Promise(resolveDone => setTimeout(resolveDone, 500));
  }
  throw Error("Timed out: " + label);
}
const evaluate = script => wd("POST", "/execute/sync", {script, args: []});
const ui = () => evaluate(
  "return {login:document.querySelector('.ipc-auth-screen')?.hidden===false," +
  "dashboard:document.querySelector('main.shell')?.hidden===false," +
  "setup:document.querySelector('.ipc-auth-screen button[type=submit]')?.textContent==='Créer mon mot de passe'," +
  "confirmHidden:document.querySelector('.ipc-auth-screen label input[autocomplete=new-password]')?.closest('label')?.hidden===true};"
);
async function element(selector) {
  const item = await wd("POST", "/element", {using:"css selector", value:selector});
  const id = item["element-6066-11e4-a52e-4f735466cecf"];
  assert.ok(typeof id === "string" && id.length, "Missing element: " + selector);
  return id;
}
async function enter(selector, value) {
  const id = await element(selector);
  await wd("POST", "/element/" + encodeURIComponent(id) + "/value",
    {text: value, value: Array.from(value)});
}
async function click(selector) {
  const id = await element(selector);
  await wd("POST", "/element/" + encodeURIComponent(id) + "/click", {});
}
const ipc = name => wd("POST", "/execute/async", {
  script: "const done=arguments[arguments.length-1];" +
    "const call=window.__TAURI__?.core?.invoke;" +
    "if(typeof call!=='function'){done({state:'missing'});return;}" +
    "Promise.resolve().then(()=>call(arguments[0]))" +
    ".then(value=>done({state:'resolved',value}))" +
    ".catch(error=>done({state:'rejected',error:String(error).slice(0,70)}));",
  args: [name]
});
async function start() {
  let occupied = false;
  try { occupied = Boolean(await http("GET", "/status", undefined, 1000)); }
  catch { /* private driver port should be unused */ }
  check("Private WebDriver port available", !occupied);
  child = spawn("tauri-driver", ["--port", String(port)], {
    windowsHide:true, stdio:["ignore","ignore","pipe"],
    env:{...process.env, PRAVDA_MARKET_FEEDS_ENABLED:"false"}
  });
  let driverError = null;
  child.on("error", error => {driverError = error;});
  child.stderr?.on("data", () => {/* never emit private paths */});
  await until(async () => {
    if (driverError) throw driverError;
    if (child.exitCode !== null) throw Error("Tauri driver exited prematurely");
    try {return Boolean(await http("GET","/status",undefined,1000));}
    catch {return false;}
  }, "tauri-driver readiness", 180);
  const opened = await http("POST", "/session", {
    capabilities:{alwaysMatch:{browserName:"wry", "tauri:options":{application:exe}}}
  }, 120000);
  session = opened?.sessionId;
  check("WebDriver launched installed production EXE",
    typeof session === "string" && session.length > 0);
  await until(async () => {
    const handles = await wd("GET", "/window/handles");
    if (!Array.isArray(handles) || !handles.length) return false;
    for (const handle of handles) {
      await wd("POST", "/window", {handle});
      if ((await wd("GET", "/title")) === "Pravda | Terminal marchés & énergie")
        return true;
    }
    return false;
  }, "real installed WebView2 window");
}
async function stop() {
  if (session) {
    try {await wd("DELETE", "");} catch {/* continue cleanup */}
    session = null;
  }
  const active = child;
  child = null;
  if (!active) return;
  if (active.exitCode === null && active.signalCode === null) {
    try {
      execFileSync("taskkill", ["/PID", String(active.pid), "/T", "/F"],
        {windowsHide:true,timeout:15000,stdio:"ignore"});
    } catch {/* driver may have already exited */}
    if (active.exitCode === null && active.signalCode === null)
      await Promise.race([
        new Promise(done => active.once("exit", done)),
        new Promise(done => setTimeout(done, 8000))
      ]);
  }
  check("WebDriver/app process tree cleaned",
    active.exitCode !== null || active.signalCode !== null);
}

try {
  assert.equal(process.platform, "win32", "Windows acceptance only");
  assert.ok(["MSI","NSIS"].includes(kind), "Installer kind required");
  assert.ok(/^[0-9a-f]{40}$/i.test(report.sha), "Exact GitHub SHA required");
  assert.ok(/^\d+\.\d+\.\d+\.\d+$/.test(report.webview2),
    "WebView2 runtime version required");
  assert.ok(report.windows, "Windows version required");
  assert.ok(Number.isInteger(port) && port >= 1024 && port <= 65535,
    "Valid private WebDriver port required");
  assert.ok(existsSync(exe) && existsSync(db) && !existsSync(auth),
    "Isolated installed app, SQLite and empty auth profile required");
  assert.equal(process.env.PRAVDA_MARKET_FEEDS_ENABLED, "false",
    "Market feeds must stay off during test");
  report.installed_exe_sha256 = digest(exe);
  const password = "Pravda-Acceptance-" + randomBytes(24).toString("hex");
  await start();
  await until(async () => (await ui()).login, "first-use setup screen");
  let state = await ui();
  check("First-use GUI demands password setup",
    state.login && !state.dashboard && state.setup && !state.confirmHidden);
  check("Protected IPC rejected before enrollment",
    (await ipc("ipc_session_check")).state === "rejected");
  await enter(".ipc-auth-screen form label:nth-of-type(1) input", password);
  await enter(".ipc-auth-screen form label:nth-of-type(2) input", password);
  await click(".ipc-auth-screen button[type=submit]");
  await until(async () => (await ui()).dashboard, "GUI password enrollment");
  check("GUI enrollment created verifier and unlocked dashboard",
    existsSync(auth) && (await ui()).dashboard);
  check("Protected IPC authorized after GUI enrollment",
    (await ipc("ipc_session_check")).state === "resolved");
  const verifier = digest(auth);
  await stop();
  await start();
  await until(async () => (await ui()).login, "restart login screen");
  state = await ui();
  check("Restart offers login and hides setup",
    state.login && !state.dashboard && !state.setup && state.confirmHidden);
  check("Restart does not inherit Rust session",
    (await ipc("ipc_session_check")).state === "rejected");
  await enter(".ipc-auth-screen input[autocomplete=current-password]", password);
  await click(".ipc-auth-screen button[type=submit]");
  await until(async () => (await ui()).dashboard, "persisted password login");
  check("Native GUI logs into installed release after restart",
    (await ipc("ipc_session_check")).state === "resolved");
  check("Password verifier unchanged after native process restart",
    digest(auth) === verifier);
  await click("#logout-button");
  await until(async () => (await ui()).login, "logout login screen");
  state = await ui();
  check("Logout visually locks dashboard", state.login && !state.dashboard);
  check("Logout revokes authenticated Rust session",
    (await ipc("ipc_session_check")).state === "rejected");
  report.passed = true;
} catch (error) {
  failure = error;
  console.error("[INSTALLED GUI ACCEPTANCE FAILED] " +
    String(error?.message || error).slice(0,220));
} finally {
  try {await stop();}
  catch (error) {failure ||= error; report.passed = false;}
  if (reportPath) {
    try {writeFileSync(reportPath, JSON.stringify(report,null,2)+"\n", {flag:"wx"});}
    catch (error) {failure ||= error; report.passed = false;}
  }
  if (failure) process.exitCode = 1;
}
