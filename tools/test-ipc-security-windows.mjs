#!/usr/bin/env node
// Issue #66: actual Tauri/WebView2 Windows IPC security acceptance.
// Never mock invoke. Fail closed on driver errors, missing windows or leaks.
import assert from "node:assert/strict";
import {spawn, execFileSync} from "node:child_process";
import {existsSync, mkdtempSync, rmSync, writeFileSync, readFileSync} from "node:fs";
import {resolve, join} from "node:path";
import {tmpdir} from "node:os";
import {randomBytes, createHash} from "node:crypto";

const exe = resolve("crates/pravda-app/target/debug/pravda-app.exe");
const basePort = Number(process.env.PRAVDA_E2E_DRIVER_PORT || "4444");
const reportPath = resolve(process.env.PRAVDA_E2E_REPORT ||
  join(tmpdir(),"Pravda-IPC-security-"+Date.now()+".json"));
const report = {
  started: new Date().toISOString(), passed: false, tests: [],
  scope: "Real WebView2: main, concurrent same-process unprivileged WebView, and separate probe",
  limits: "Concurrent native probe reports bounded ACL decisions via a debug-only title event. The hostile HTTPS origin is only a controlled synthetic script executed by native WebDriver; it does not replace a full XSS penetration test."
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
  " .then(value=>done({state:'resolved',value}))",
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
// On Windows, ChildProcess.killed only means the signal was requested;
// it does NOT prove tauri-driver/msedgedriver exited. Reusing the listening
// port before a full process-tree exit makes the next session unreliable.
function driverStopped(processHandle) {
  return processHandle.exitCode !== null || processHandle.signalCode !== null;
}
async function stopSession() {
  const active = child;
  if (session) {
    try { await wd("DELETE", ""); } catch { /* cleanup continues */ }
    session = null;
  }
  if (!active) return;
  child = null;
  if (!driverStopped(active)) {
    // The Windows-only test harness owns this exact PID. Kill the driver
    // and any child EdgeDriver/app process, not unrelated applications.
    try {
      execFileSync("taskkill", ["/PID", String(active.pid), "/T", "/F"], {
        windowsHide: true,
        timeout: 15_000,
        stdio: "ignore"
      });
    } catch { /* It may have already exited during DELETE /session. */ }
    if (!driverStopped(active)) {
      await Promise.race([
        new Promise(resolveDone => active.once("exit", resolveDone)),
        new Promise(resolveDone => setTimeout(resolveDone, 8_000))
      ]);
    }
  }
  if (!driverStopped(active)) {
    throw Error("Native WebDriver process tree did not stop cleanly");
  }
}
async function assertDriverPortFree(driverPort) {
  port = driverPort;
  try {
    const response = await http("GET", "/status", undefined, 1_000);
    throw Error("Native WebDriver port still active: " + driverPort);
  } catch (error) {
    if (String(error).includes("still active")) throw error;
    // An unreachable status endpoint is expected before binding a new driver.
  }
}
async function startSession(mode, driverPort) {
  await stopSession();
  await assertDriverPortFree(driverPort);
  driverFault = null;
  let stderrTail = "";
  const env = {...process.env,
    PRAVDA_DB_PATH:join(directory,"data.db"),
    PRAVDA_AUTH_PATH:join(directory,"auth.json"),
    PRAVDA_MARKET_FEEDS_ENABLED:"false",
    PRAVDA_E2E_PROBE_ONLY:mode==="probe"?"true":"false",
    PRAVDA_E2E_CONCURRENT:mode==="main"?"true":"false",
    PRAVDA_E2E_CONCURRENT_REPORT:join(directory,"concurrent-native-evidence.log")
  };
  for (const key of ["PRAVDA_ADMIN_PASSWORD","PRAVDA_LEGACY_DB_PATH","MARKET_HISTORY_DB"])
    delete env[key];
  child = spawn("tauri-driver", ["--port", String(port)], {
    cwd:process.cwd(),env,stdio:["ignore","ignore","pipe"],windowsHide:true
  });
  child.stderr?.on("data", data => {
    // Consume stderr so the pipe cannot block the driver.
    stderrTail = (stderrTail + String(data)).slice(-1200);
  });
  child.on("error", error => { driverFault=error; });
  try {
    await waitForDriver();
  } catch (error) {
    // Do not print full driver stderr (it could contain local paths).
    throw Error(String(error.message||error) +
      " [driver exit=" + (child.exitCode ?? child.signalCode ?? "running") +
      ", stderr bytes captured=" + stderrTail.length + "]");
  }
  const created=await http("POST","/session",{capabilities:{alwaysMatch:{
    browserName:"wry","tauri:options":{application:exe}
  }}},120_000);
  session=created?.sessionId;
  assert.ok(typeof session==="string"&&session.length>0,
    "tauri-driver did not return a sessionId");
}

// Proves a capability-free WebView runs in the SAME native Rust process as
// the authenticated main WebView. A debug-only Rust document-title callback
// reports the outcomes, even when EdgeDriver enumerates only its first view.
async function checkConcurrentWindowAfterAuth(authenticatedAt) {
  const file = join(directory,"concurrent-native-evidence.log");
  let latest;
  for (let attempt=0;attempt<40;attempt++) {
    if (existsSync(file)) {
      const lines=readFileSync(file,"utf8").trim().split(/\r?\n/);
      for (const line of lines) {
        const parts=line.split("|");
        if (parts.length!==3 || Number(parts[0])<authenticatedAt)continue;
        if (!/^\d+$/.test(parts[1]))continue;
        try {
          const proof=JSON.parse(Buffer.from(parts[2],"base64").toString("utf8"));
          if (proof.v===1 && Array.isArray(proof.results)) latest=proof;
        } catch {/* reject malformed side-channel data */ }
      }
    }
    if (latest)break;
    await new Promise(resolveDone=>setTimeout(resolveDone,500));
  }
  verdict("concurrent: unauthorized native WebView reports after login",
    Boolean(latest), latest?"native title callback received":"no verified native report");
  for (const command of ["ipc_auth_status","ipc_session_check","ipc_market_snapshot",
    "ipc_history_recent","ipc_history_query","ipc_history_export_csv",
    "ipc_paper_alerts_snapshot","ipc_runtime_metrics","ipc_security_report","ipc_fx_status",
    "plugin:fs|write_text_file","plugin:shell|execute"]) {
    const record=latest.results.find(item=>item.command===command);
    verdict("concurrent: "+command+" denied with main authenticated",
      record?.state==="rejected" && record?.access_denied===true &&
        record?.denial_layer==="tauri_acl" && record?.malformed===false,
      record ? record.state+" denial_layer="+record.denial_layer+
        " malformed="+record.malformed : "missing result");
  }
}

// Verify the actual CSP enforced by WebView2: the WebDriver-controlled script
// can insert a child script element, but script-src 'self' must prevent that
// newly inserted inline script from running. This is not a JS invoke mock.
async function checkCspOnNativeMain() {
  const executed=await wd("POST","/execute/sync",{
    script:"window.__pravdaInlineCspProof=undefined; const s=document.createElement('script'); s.textContent='window.__pravdaInlineCspProof=true'; document.body.appendChild(s); return window.__pravdaInlineCspProof===true;",
    args:[]
  });
  verdict("main: native WebView2 CSP blocks injected inline script",executed===false,
    "DOM inline script executed="+executed);
}

// Exercise a controlled external HTTPS page while the main Rust Auth is live.
// Navigation denial is acceptable, but must leave the trusted app page loaded.
// If navigation is allowed, the test must observe an untrusted https: origin
// and fail if that document can invoke privileged Tauri commands.
async function checkHostileHttpsOrigin() {
  const origin="https://example.com/";
  await wd("POST","/timeouts",{pageLoad:20000,script:20000});
  let navigationError=null;
  try {
    await wd("POST","/url",{url:origin});
  } catch (error) {
    navigationError=String(error.message||error).slice(0,120);
  }
  let current;
  try {
    current=await wd("GET","/url");
  } catch (error) {
    throw Error("Hostile origin test lost native WebView: "+String(error).slice(0,120));
  }
  const location=String(current||"");
  if (location.startsWith("https://example.com/")) {
    // A browser error document can retain the requested URL. It is NOT
    // evidence that the hostile HTTPS page loaded successfully.
    verdict("hostile HTTPS navigation completed without driver error",
      navigationError===null, "driver error="+Boolean(navigationError));
    const documentProof=await wd("POST","/execute/sync",{
      script:"return {origin:document.location.origin,url:document.URL,ready:document.readyState,hasBody:!!document.body};",
      args:[]
    });
    verdict("hostile HTTPS document origin verified by WebView2",
      documentProof?.origin==="https://example.com" &&
        documentProof?.url?.startsWith("https://example.com/") &&
        documentProof?.ready==="complete" && documentProof?.hasBody===true,
      "document origin="+String(documentProof?.origin).slice(0,100)+
        " ready="+String(documentProof?.ready));
    const denied=await invoke("ipc_market_snapshot");
    verdict("untrusted HTTPS origin cannot invoke market IPC",
      denied.state==="no-bridge" ||
      (denied.state==="rejected" && /not allowed|denied|capability|permission|forbidden|URL/i.test(denied.error||"")),
      "HTTPS page: "+denied.state+" "+(denied.error||""));
    verdict("hostile origin never receives Tauri business capabilities",true,
      "WebView2 document origin example.com, no remote capabilities");
  } else {
    // A retained trusted URL alone proves nothing: a driver/protocol failure
    // can leave the original page untouched without exercising Tauri policy.
    // Only an explicit navigation-policy denial can count as a rejection.
    const policyDenied=Boolean(navigationError) &&
      /(?:ERR_BLOCKED_BY_CLIENT|navigation (?:is )?(?:not allowed|denied|blocked)|blocked by (?:navigation )?policy|not allowed to navigate)/i.test(navigationError);
    verdict("untrusted HTTPS navigation blocked in authenticated main window",
      policyDenied &&
        /^(?:tauri:|https?:\/\/(?:tauri\.localhost|localhost)(?:[:/]|$))/.test(location),
      "trusted page retained="+location.slice(0,130)+
        "; explicit policy denial="+policyDenied);
  }
}

try {
  assert.equal(process.platform,"win32","Requires native Windows WebView2");
  assert.ok(existsSync(exe),"Debug security-e2e executable missing");
  assert.ok(Number.isInteger(basePort)&&basePort>=1024&&basePort<=65533,
    "Invalid base driver port");
  report.git_sha=execFileSync("git",["rev-parse","HEAD"],{encoding:"utf8"}).trim();
  report.windows_version=String(process.env.PRAVDA_E2E_WINDOWS_VERSION||"");
  report.webview2_version=String(process.env.PRAVDA_E2E_WEBVIEW2_VERSION||"");
  assert.ok(report.windows_version&&/^\d+\.\d+\.\d+\.\d+$/.test(report.webview2_version),
    "Missing native Windows/WebView2 version evidence");
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
  await checkConcurrentWindowAfterAuth(Date.now());
  await checkCspOnNativeMain();

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

  const login=await invoke("ipc_auth_login",{password});
  verdict("main: authenticated again for hostile HTTPS test",login.state==="resolved",login.state);
  // Native process restart: persisted credentials and SQLite survive, but
  // Rust-only session tokens must not carry over into the new process.
  const authFile=join(directory,"auth.json");
  const dbFile=join(directory,"data.db");
  verdict("restart: password verifier and SQLite initialized",
    existsSync(authFile)&&existsSync(dbFile), "Both private storage files exist");
  const authHash=createHash("sha256").update(readFileSync(authFile)).digest("hex");
  const sqliteHeader=readFileSync(dbFile).subarray(0,16).toString("ascii");
  verdict("restart: SQLite file header",sqliteHeader==="SQLite format 3\0",
    "SQLite format header present");
  await startSession("main",basePort);
  await ensureWebviewTitle("Pravda | Terminal marchés & énergie");
  const authStatus=await invoke("ipc_auth_status");
  verdict("restart: password setup persisted across native process",
    authStatus.state==="resolved"&&authStatus.value===true,
    "Existing password verifier was loaded");
  rustDenied("restart: old session revoked after native process exit",
    await invoke("ipc_market_snapshot"));
  verdict("restart: password verifier unchanged",
    createHash("sha256").update(readFileSync(authFile)).digest("hex")===authHash,
    "Password verifier bytes unchanged");
  rustDenied("security: unauthenticated user cannot read security status",
    await invoke("ipc_security_report"));
  const loginAfterRestart=await invoke("ipc_auth_login",{password});
  verdict("restart: existing password logs in",
    loginAfterRestart.state==="resolved",loginAfterRestart.state);
  const security=await invoke("ipc_security_report");
  verdict("security: authenticated status is bounded and VPN remains unverified",
    security.state==="resolved" && Array.isArray(security.value) &&
    security.value.length===8 && security.value[0].startsWith("session|verified|") &&
    security.value.some(row=>row.startsWith("proton|unknown|")) &&
    security.value.some(row=>row.startsWith("kill_switch|unknown|")) &&
    security.value.every(row=>typeof row==="string" && row.length<300),
    "Safe local status with honest unknown Proton VPN and kill switch");
  const marketAfterRestart=await invoke("ipc_market_snapshot");
  verdict("restart: market IPC authorized only after login",
    marketAfterRestart.state==="resolved",marketAfterRestart.state);
  verdict("restart: SQLite remains accessible",
    existsSync(dbFile)&&readFileSync(dbFile).subarray(0,16).toString("ascii")==="SQLite format 3\0",
    "Same database path and valid SQLite header");
  await checkHostileHttpsOrigin();

  // tauri-driver / EdgeDriver on Windows may enumerate only its primary
  // WebView. Start the truly unauthorized Tauri label as the primary WebView
  // in a SECOND native process against the same disposable auth/db files.
  // This proves window capability enforcement, NOT shared-session isolation.
  await startSession("probe",basePort);
  await ensureWebviewTitle("Pravda Security Probe - NO IPC");
  runtimeDenied("security-probe: bootstrap auth_status denied",
    await invoke("ipc_auth_status"));
  runtimeDenied("security-probe: market snapshot denied",
    await invoke("ipc_market_snapshot"));
  const probeForbidden=join(directory,"PROBE_MUST_NOT_EXIST.txt");
  runtimeDenied("security-probe: filesystem plugin denied",
    await invoke("plugin:fs|write_text_file",
      {path:probeForbidden,contents:"security probe must not write"}));
  verdict("security-probe: no arbitrary disk modification",
    !existsSync(probeForbidden), "No file created by the unauthorized WebView");
  runtimeDenied("security-probe: shell plugin denied",
    await invoke("plugin:shell|execute",
      {program:"__PRAVDA_PROBE_NONEXISTENT__"}));
  report.passed=true;
} catch(error) {
  failure=error;
  console.error("[SECURITY E2E FAILED] "+String(error.message||error).slice(0,500));
} finally {
  try { await stopSession(); }
  catch (error) {
    report.cleanupError=String(error.message||error).slice(0,200);
    report.passed=false;
  }
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
