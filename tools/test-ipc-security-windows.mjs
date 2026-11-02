#!/usr/bin/env node
// Issue #66: actual Tauri/WebView2 Windows IPC security acceptance.
// Never mock invoke. Fail closed on driver errors, missing windows or leaks.
import assert from "node:assert/strict";
import {spawn, execFileSync} from "node:child_process";
import {existsSync, mkdtempSync, rmSync, writeFileSync} from "node:fs";
import {resolve, join} from "node:path";
import {tmpdir} from "node:os";
import {randomBytes} from "node:crypto";

const exe = resolve("crates/pravda-app/target/debug/pravda-app.exe");
const basePort = Number(process.env.PRAVDA_E2E_DRIVER_PORT || "4444");
const reportPath = resolve(process.env.PRAVDA_E2E_REPORT ||
  join(tmpdir(),"Pravda-IPC-security-"+Date.now()+".json"));
const report = {
  started: new Date().toISOString(), passed: false, tests: [],
  scope: "Two separate real WebView2 sessions: main and capability-free probe",
  limits: "Probe is launched as the primary WebView in a second debug-only process; this does not demonstrate cross-window auth sharing or remote-origin enforcement."
};
let port, child, session, directory, driverFault, failure;

function verdict(name, passed, evidence) {
  const safe = String(evidence).slice(0, 400);
  report.tests.push({name,passed,evidence:safe});
  console.log("["+(passed?"PASS":"FAIL")+"] "+name+" — "+safe);
  assert.ok(passed, name+": "+safe);
}
async function http(method,path,body,timeout=20_000) {
  const response=await fetch("http://127.0.0.1:"+port+path,{
    method,headers:{"content-type":"application/json"},
    body:body===undefined?undefined:JSON.stringify(body),
    signal:AbortSignal.timeout(timeout)
  });
  const data=await response.json();
  // Application-level IPC rejections are successful WebDriver replies:
  // value={state:"rejected",error:"UNAUTHORIZED"}. Only a W3C WebDriver
  // protocol error has BOTH error and message, typically with an HTTP 4xx/5xx.
  const wdError = typeof data.value?.error === "string" &&
    typeof data.value?.message === "string";
  if(!response.ok || wdError) {
    throw Error("WebDriver "+method+" "+path+": "+
      String(wdError ? data.value.error+": "+data.value.message :
        response.status).slice(0,220));
  }
  return data.value;
}
const wd=(method,path,body)=>http(method,"/session/"+session+path,body);

async function waitForDriver() {
  for(let attempt=0;attempt<180;attempt++) {
    if(driverFault)throw driverFault;
    if(child.exitCode!==null)throw Error("tauri-driver exited before readiness");
    try{
      const value=await http("GET","/status",undefined,1500);
      if(value?.ready!==false)return;
    }catch{/* wait until live */}
    await new Promise(r=>setTimeout(r,500));
  }
  throw Error("tauri-driver startup timeout");
}
const js=[
  "const done=arguments[arguments.length-1];",
  "const call=window.__TAURI__?.core?.invoke;",
  "if(typeof call!=='function'){done({state:'no-bridge'});return;}",
  "Promise.resolve().then(()=>call(arguments[0],arguments[1]))",
  " .then(()=>done({state:'resolved'}))",
  " .catch(error=>done({state:'rejected',error:String(error).slice(0,250)}));"
].join("\n");

const invoke=(name,args={})=>wd("POST","/execute/async",
  {script:js,args:[name,args]});

function rustDenied(name, value) {
  verdict(name,value.state==="rejected"&&/UNAUTHORIZED/.test(value.error||""),
    value.state+" "+(value.error||""));
}
function runtimeDenied(name,value) {
  // Explicit runtime rejection, not a missing API silently counted as pass.
  verdict(name,value.state==="rejected",
    value.state+" "+(value.error||""));
}
async function windowTitle() {
  const handles=await wd("GET","/window/handles");
  assert.ok(Array.isArray(handles)&&handles.length>0,"No real WebView2 window handles");
  const titles=[];
  for(const handle of handles) {
    await wd("POST","/window",{handle});
    titles.push({handle,title:await wd("GET","/title")});
  }
  return titles;
}
async function ensureWebviewTitle(title) {
  let found=[];
  for(let i=0;i<60;i++) {
    found=await windowTitle();
    const match=found.find(w=>w.title===title);
    if(match) {
      await wd("POST","/window",{handle:match.handle});
      verdict("WebView2 document: "+title,true,
        "Native WebDriver handle with expected HTML title");
      return;
    }
    await new Promise(r=>setTimeout(r,500));
  }
  verdict("WebView2 document: "+title,false,
    "Observed: "+found.map(x=>x.title).join(" | "));
}
async function stopSession() {
  if(session) {
    try{await wd("DELETE","");}catch{/* best effort */}
    session=null;
  }
  if(child) {
    const old=child;
    old.kill();
    await new Promise(resolveDone=>{
      if(old.exitCode!==null||old.killed) return resolveDone();
      const done=()=>resolveDone();
      old.once("exit",done);
      setTimeout(done,2500).unref();
    });
    child=null;
  }
}
async function startSession(mode,driverPort) {
  await stopSession();
  port=driverPort;
  driverFault=null;
  const env={...process.env,
    PRAVDA_DB_PATH:join(directory,"data.db"),
    PRAVDA_AUTH_PATH:join(directory,"auth.json"),
    PRAVDA_MARKET_FEEDS_ENABLED:"false",
    PRAVDA_E2E_PROBE_ONLY:mode==="probe"?"true":"false"
  };
  for(const key of ["PRAVDA_ADMIN_PASSWORD","PRAVDA_LEGACY_DB_PATH","MARKET_HISTORY_DB"])
    delete env[key];
  child=spawn("tauri-driver",["--port",String(port)],{
    cwd:process.cwd(),env,stdio:["ignore","ignore","pipe"],windowsHide:true
  });
  child.on("error",err=>{driverFault=err;});
  await waitForDriver();
  const created=await http("POST","/session",{capabilities:{alwaysMatch:{
    browserName:"wry","tauri:options":{application:exe}
  }}},120_000);
  session=created?.sessionId;
  assert.ok(typeof session==="string"&&session.length>0,
    "tauri-driver did not return a sessionId");
}
try {
  assert.equal(process.platform,"win32","Requires native Windows WebView2");
  assert.ok(existsSync(exe),"Debug security-e2e executable missing");
  assert.ok(Number.isInteger(basePort)&&basePort>=1024&&basePort<=65533,
    "Invalid base driver port");
  report.git_sha=execFileSync("git",["rev-parse","HEAD"],{encoding:"utf8"}).trim();
  directory=mkdtempSync(join(tmpdir(),"pravda-ipc-e2e-"));

  await startSession("main",basePort);
  await ensureWebviewTitle("Pravda | Terminal marchés & énergie");

  for(const name of ["ipc_market_snapshot","ipc_history_recent",
    "ipc_paper_alerts_snapshot","ipc_runtime_metrics",
    "ipc_fx_status","ipc_session_check"])
    rustDenied("no session: "+name,await invoke(name));
  const query={symbol:"BTC/USDT",exchange:"Binance",period:"day"};
  for(const name of ["ipc_history_query","ipc_history_export_csv"])
    rustDenied("no session: "+name,await invoke(name,{query}));

  const password="Pravda-E2E-"+randomBytes(32).toString("hex");
  const setup=await invoke("ipc_auth_setup",{password,confirm:password});
  verdict("main: one-time test setup",setup.state==="resolved",setup.state);
  const market=await invoke("ipc_market_snapshot");
  verdict("main: authenticated snapshot",market.state==="resolved",market.state);

  const forbidden=join(directory,"MUST_NOT_EXIST.txt");
  runtimeDenied("main: filesystem plugin denied",
    await invoke("plugin:fs|write_text_file",
      {path:forbidden,contents:"test only"}));
  verdict("no arbitrary filesystem write",!existsSync(forbidden),
    "No test file created");
  runtimeDenied("main: shell plugin denied",
    await invoke("plugin:shell|execute",
      {program:"__PRAVDA_TEST_NO_PROGRAM__"}));

  const logout=await invoke("ipc_auth_logout");
  verdict("main: logout",logout.state==="resolved",logout.state);
  rustDenied("main: logout revokes market access",
    await invoke("ipc_market_snapshot"));

  // tauri-driver / EdgeDriver on Windows may enumerate only its primary
  // WebView. Start the truly unauthorized Tauri label as the primary WebView
  // in a SECOND native process against the same disposable auth/db files.
  // This proves window capability enforcement, NOT shared-session isolation.
  await startSession("probe",basePort+1);
  await ensureWebviewTitle("Pravda Security Probe - NO IPC");
  runtimeDenied("security-probe: bootstrap auth_status denied",
    await invoke("ipc_auth_status"));
  runtimeDenied("security-probe: market snapshot denied",
    await invoke("ipc_market_snapshot"));
  report.passed=true;
} catch(error) {
  failure=error;
  console.error("[SECURITY E2E FAILED] "+String(error.message||error).slice(0,500));
} finally {
  await stopSession();
  if(directory) {
    try{rmSync(directory,{recursive:true,force:true});}
    catch(error){
      report.cleanupError=String(error.message||error).slice(0,200);
      report.passed=false;
    }
  }
  report.finished=new Date().toISOString();
  if(failure)report.error=String(failure.message||failure).slice(0,500);
  writeFileSync(reportPath,JSON.stringify(report,null,2)+"\n",{flag:"wx"});
  console.log("Security evidence report (no passwords): "+reportPath);
}
if(!report.passed)process.exitCode=1;
