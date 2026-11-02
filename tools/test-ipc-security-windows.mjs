#!/usr/bin/env node
// Issue #66: native Tauri/WebView2 security acceptance. No mock invoke.
import assert from "node:assert/strict";
import {spawn, execFileSync} from "node:child_process";
import {existsSync, mkdtempSync, rmSync, writeFileSync} from "node:fs";
import {resolve, join} from "node:path";
import {tmpdir} from "node:os";
import {randomBytes} from "node:crypto";

const exe = resolve("crates/pravda-app/target/debug/pravda-app.exe");
const port = Number(process.env.PRAVDA_E2E_DRIVER_PORT || "4444");
const endpoint = "http://127.0.0.1:" + port;
const reportPath = resolve(process.env.PRAVDA_E2E_REPORT ||
  join(tmpdir(), "Pravda-IPC-security-" + Date.now() + ".json"));
const report = {started:new Date().toISOString(), passed:false, tests:[],
  scope:"native Windows WebView2 IPC, no mocked APIs",
  limits:"Remote-origin navigation and Windows ACL permissions not covered."};
let child, session, directory, problem;

function verdict(name, pass, evidence) {
  report.tests.push({name, pass, evidence});
  console.log("[" + (pass ? "PASS" : "FAIL") + "] " + name + " — " + evidence);
  assert.ok(pass, name + ": " + evidence);
}
async function http(method, path, payload, ms=20000) {
  const response = await fetch(endpoint + path, {method,
    headers:{"content-type":"application/json"},
    body:payload===undefined?undefined:JSON.stringify(payload),
    signal:AbortSignal.timeout(ms)});
  const data = await response.json();
  if(!response.ok || data.value?.error) {
    throw new Error("WebDriver " + method + " " + path + ": " +
      String(data.value?.message || response.status).slice(0,220));
  }
  return data.value;
}
async function waitForDriver() {
  for(let attempt=0;attempt<180;attempt++) {
    if(child.exitCode!==null) throw new Error("tauri-driver exited prematurely");
    try {
      const s=await http("GET","/status",undefined,1500);
      if(s?.ready!==false)return;
    } catch { /* not ready */ }
    await new Promise(r=>setTimeout(r,500));
  }
  throw new Error("tauri-driver readiness timeout");
}
const wd=(m,path,payload)=>http(m,"/session/"+session+path,payload);
const invokeScript=[
  "const done=arguments[arguments.length-1];",
  "const call=window.__TAURI__?.core?.invoke;",
  "if(typeof call!=='function'){done({state:'no-bridge'});return;}",
  "Promise.resolve().then(()=>call(arguments[0],arguments[1]))",
  " .then(()=>done({state:'resolved'}))",
  " .catch(e=>done({state:'rejected',error:String(e).slice(0,250)}));"
].join("\n");
const invoke=(name,args={})=>wd("POST","/execute/async",
  {script:invokeScript,args:[name,args]});
function rustDenied(name,r) {
  verdict(name,r.state==="rejected"&&/UNAUTHORIZED/.test(r.error||""),
    r.state+" "+(r.error||""));
}
function aclDenied(name,r) {
  verdict(name,r.state==="rejected"||r.state==="no-bridge",
    r.state+" "+(r.error||""));
}
async function select(handle) {await wd("POST","/window",{handle});}
async function windows() {
  const handles=await wd("GET","/window/handles");
  const result=[];
  for(const handle of handles){await select(handle);result.push({handle,title:await wd("GET","/title")});}
  return result;
}
try {
  assert.equal(process.platform,"win32","Windows required");
  assert.ok(existsSync(exe),"Missing debug EXE; build --features security-e2e");
  assert.ok(Number.isInteger(port)&&port>=1024&&port<=65535,"Bad port");
  report.sha=execFileSync("git",["rev-parse","HEAD"],{encoding:"utf8"}).trim();

  // Refuse to attach to a pre-existing driver from a different task.
  try {
    await http("GET","/status",undefined,1000);
    throw new Error("Driver port already in use; choose PRAVDA_E2E_DRIVER_PORT");
  } catch(e) {
    if(String(e).includes("already in use"))throw e;
  }
  directory=mkdtempSync(join(tmpdir(),"pravda-security-"));
  const env={...process.env,PRAVDA_DB_PATH:join(directory,"data.db"),
    PRAVDA_AUTH_PATH:join(directory,"auth.json"),
    PRAVDA_MARKET_FEEDS_ENABLED:"false"};
  for(const k of ["PRAVDA_ADMIN_PASSWORD","PRAVDA_LEGACY_DB_PATH","MARKET_HISTORY_DB"])
    delete env[k];
  child=spawn("tauri-driver",["--port",String(port)],{
    cwd:process.cwd(),env,stdio:["ignore","ignore","pipe"],windowsHide:true});
  child.on("error",e=>{problem=e;});
  await waitForDriver();
  if(problem)throw problem;
  const created=await http("POST","/session",{capabilities:{alwaysMatch:{
    browserName:"wry","tauri:options":{application:exe}}}},120000);
  session=created?.sessionId;
  assert.ok(session,"No native Tauri WebDriver session");
  const found=await windows();
  const main=found.find(w=>w.title==="Pravda-Bot");
  const probe=found.find(w=>w.title.includes("Pravda-Bot Security Probe"));
  verdict("actual main and unprivileged WebViews",
    !!(main&&probe&&main.handle!==probe.handle),
    found.map(w=>w.title).join(" | "));
  await select(main.handle);

  for(const name of ["ipc_market_snapshot","ipc_history_recent",
    "ipc_paper_alerts_snapshot","ipc_runtime_metrics","ipc_fx_status",
    "ipc_session_check"])rustDenied("no session: "+name,await invoke(name));
  const query={symbol:"BTC/USDT",exchange:"Binance",period:"day"};
  for(const name of ["ipc_history_query","ipc_history_export_csv"])
    rustDenied("no session: "+name,await invoke(name,{query}));

  const password="Pravda-E2E-"+randomBytes(32).toString("hex");
  const setup=await invoke("ipc_auth_setup",{password,confirm:password});
  verdict("test-only setup",setup.state==="resolved",setup.state);
  const valid=await invoke("ipc_market_snapshot");
  verdict("authenticated main",valid.state==="resolved",valid.state);

  await select(probe.handle);
  aclDenied("other window: auth_status",await invoke("ipc_auth_status"));
  aclDenied("other window: market snapshot",await invoke("ipc_market_snapshot"));

  await select(main.handle);
  const forbidden=join(directory,"MUST_NOT_EXIST.txt");
  aclDenied("no fs write plugin",await invoke("plugin:fs|write_text_file",
    {path:forbidden,contents:"test"}));
  verdict("no unauthorized file created",!existsSync(forbidden),"File absent");
  aclDenied("no shell execution plugin",await invoke("plugin:shell|execute",
    {program:"__PRAVDA_NONEXISTENT_TEST__"}));

  const logout=await invoke("ipc_auth_logout");
  verdict("logout",logout.state==="resolved",logout.state);
  rustDenied("after logout: market",await invoke("ipc_market_snapshot"));
  report.passed=true;
} catch(e) {
  problem=e;
  console.error("[SECURITY E2E FAILED] "+String(e.message||e).slice(0,500));
} finally {
  if(session)try{await wd("DELETE","");}catch{}
  if(child)child.kill();
  if(directory)rmSync(directory,{recursive:true,force:true});
  report.finished=new Date().toISOString();
  if(problem)report.error=String(problem.message||problem).slice(0,500);
  writeFileSync(reportPath,JSON.stringify(report,null,2)+"\n",{flag:"wx"});
  console.log("Evidence report (no credentials): "+reportPath);
}
if(!report.passed)process.exitCode=1;
