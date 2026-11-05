// Contract checks for IPC-only desktop. Native Windows E2E remains required.
import {readFileSync} from "node:fs";
import {join} from "node:path";
import assert from "node:assert/strict";
import vm from "node:vm";
const root=join(process.cwd(),"crates/pravda-app");
const read=p=>readFileSync(join(root,p),"utf8");
const cfg=JSON.parse(read("tauri.conf.json"));
const cap=JSON.parse(read("capabilities/main.json"));
const main=read("src/main.rs"), ipc=read("src/ipc.rs"), paper=read("src/paper.rs"), build=read("build.rs");
const html=read("frontend/index.html"), js=read("frontend/dashboard.js"),auth=read("frontend/auth.js"), css=read("frontend/styles.css");
const installerWorkflow=readFileSync(join(process.cwd(),".github/workflows/windows-package.yml"),"utf8");
const msiAcceptance=readFileSync(join(process.cwd(),"tools/Test-Pravda-Msi-Install.ps1"),"utf8");
const nsisAcceptance=readFileSync(join(process.cwd(),"tools/Test-Pravda-Nsis-Install.ps1"),"utf8");
const setupSource=readFileSync(join(process.cwd(),"rust/pravda-server/src/bin/pravda-setup.rs"),"utf8");
for (const [type, acceptance] of [["MSI",msiAcceptance],["NSIS",nsisAcceptance]]) {
  assert.match(acceptance,/installed release desktop restarts with its existing SQLite and no implicit auth/,
    type+": packaged desktop must survive process restart on the same SQLite");
  assert.match(acceptance,/SQLite format 3/,
    type+": post-restart SQLite signature must be checked");
  assert.match(acceptance,/secondListeners/,
    type+": restarted installed desktop must not expose a TCP listener");
}
assert.equal(cfg.build.frontendDist,"frontend");
assert.deepEqual([...cfg.bundle.targets].sort(),["msi","nsis"],
  "Both MSI and NSIS must be produced for Windows release");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.type,"offlineInstaller",
  "Windows installers must contain the official offline WebView2 prerequisite");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.silent,true,
  "WebView2 must install automatically, without user-run commands");
assert.match(installerWorkflow,/Smoke-test MSI installation and uninstall on pristine Windows/);
assert.match(installerWorkflow,/pravda-windows-single-file-setup/,
  "Single user-facing setup must be published independently from CI artifacts");
assert.match(setupSource,/PathBuf::from\(system_root\)\s*\.join\("System32"\)\s*\.join\("msiexec\.exe"\)/,
  "Installer must use SystemRoot/System32 instead of an untrusted PATH executable");
assert.match(setupSource,/Command::new\(msiexec\)/,
  "User-facing setup must invoke the actual Windows Installer MSI program");
assert.doesNotMatch(setupSource,/launch_diagnostic|const CENTER|six-second/i,
  "Installer must not substitute the temporary diagnostic desktop for the installed application");
assert.match(installerWorkflow,/Smoke-test NSIS installation and uninstall on pristine Windows/);
assert.match(installerWorkflow,/Test-Pravda-Nsis-Install\.ps1/,
  "Release gate must execute the actual NSIS installer in a disposable Windows environment");

assert.equal(cfg.app.withGlobalTauri,true);
assert.deepEqual(cfg.app.security.capabilities,["main-ipc"]);
assert.match(cfg.app.security.csp["connect-src"],/ipc:/);
assert.doesNotMatch(cfg.app.security.csp["script-src"],/unsafe-inline|https?:/);
assert.deepEqual(cap.windows,["main"]);
assert.match(main, /#\[cfg\(all\(windows, debug_assertions, feature = "security-e2e"\)\)\]/);
assert.match(main, /"security-probe"/);
assert.ok(!cap.windows.includes("security-probe"),"Security test window must have zero capabilities");

assert.ok(!("remote" in cap));
assert.ok(cap.permissions.every(p=>/^allow-ipc-[a-z-]+$/.test(p)));
assert.ok(!/fs:|shell:|\*/.test(cap.permissions.join(",")));
assert.match(main,/WebviewUrl::App\("index\.html"\.into\(\)\)/);
assert.doesNotMatch(main,/WebviewUrl::External|TcpListener|run_server_with_auth|run_server\(|reqwest::/);
assert.match(main,/\.manage\(desktop\)/);
assert.doesNotMatch(html,/<script(?![^>]*\bsrc=)|<style\b|style="/i);
assert.match(html,/src="\.\/auth\.js"/);
assert.match(auth,/setup = false/);
assert.match(css,/\.ipc-auth-screen\[hidden\]/);
assert.match(auth,/if \(loginVisible\) return;/);
assert.match(css,/\.ipc-auth-card label\[hidden\]/);
assert.doesNotMatch(js,/\bfetch\s*\(|\bnew\s+WebSocket\s*\(|location\.assign\s*\(/);
assert.match(html,/id="paper-alerts"/,"Paper-simulation events need a separate renderer");
assert.match(html,/data-tab="security"/,"Security status must be visible to an authenticated user");
assert.match(html,/data-tab="security"[^>]*>Paramètres · Sécurité &amp; dépannage<\//,
  "Settings must expose the authenticated Security and Troubleshooting hub");
assert.match(html,/id="security-refresh"[^>]*>Relancer les tests locaux<\//,
  "User must explicitly be able to rerun existing read-only IPC security checks");
assert.match(html,/data-go-tab="system">Consulter les métriques<\//,
  "Maintenance must reuse the existing Rust runtime metrics, not spawn a shell");
assert.match(html,/id="security-events"/,
  "Maintenance must retain bounded in-memory security event viewer");
assert.match(html,/id="acceptance-center"/,
  "Installed desktop must own a native test center");
assert.match(html,/id="acceptance-pre"/, "Preproduction button must exist");
assert.match(html,/id="acceptance-post"/, "Postproduction button must exist");
assert.match(js,/window\.pravdaInvoke\("ipc_acceptance_report", \{phase\}\)/,
  "Pre/postproduction tests must be native authenticated Rust IPC");
assert.match(auth,/window\.pravdaClearAcceptance\(\)/,
  "Logout must erase old-session acceptance results");
assert.doesNotMatch(html,/centre externe|pravda-test-center\.exe/i,
  "User-facing app must not redirect to a separate diagnostic executable");

assert.match(js,/ipc_security_report/,"Security UI must retrieve an authenticated Rust-owned report");
assert.match(js,/ipc_security_events/,"Security events must remain authenticated Tauri IPC only");
assert.match(html,/id="paper-sandbox"/,"Paper mode must be accessible from authenticated settings");
assert.match(html,/id="paper-order-form"/,"Paper orders must have a usable form");
assert.match(html,/cotations fixes et fictives|Cotations fixes et fictives/,
  "Paper-only quotes must never be mislabeled as live market data");
assert.match(main,/\.manage\(paper::PaperSandbox::default\(\)\)/,
  "Native session must own the independent synthetic portfolio");
assert.match(auth,/window\.refreshPaper\(\)/,
  "Reauthentication must refresh the synthetic portfolio");
assert.match(auth,/window\.pravdaClearPaper\(\)/,
  "Logout and reauthentication must erase previous paper values from the DOM");
assert.match(js,/window\.pravdaClearPaper\s*=\s*clearPaperUi/,
  "Paper UI must expose a local clear method without contacting exchanges");
assert.match(js,/if \(generation !== window\.pravdaAuthGeneration\) refreshPaper\(\);/,
  "Stale paper action completion must refresh after a new login");
assert.match(js,/window\.pravdaInvoke\(command, args\)/,
  "Paper trading adapter must call native Rust IPC, never fetch or WebSocket");
assert.match(js,/paperAction\("ipc_paper_trade"/,
  "Paper order submit must be wired");
assert.match(js,/paperAction\("ipc_paper_reset"\)/,
  "Paper reset must be wired");
assert.match(paper,/START_CENTS:\s*u64\s*=\s*1_000_000/,
  "Synthetic initial balance must be deterministic and declared");
assert.match(paper,/FEE_BPS:\s*u64\s*=\s*36/,
  "Fixed paper fees must be explicit");
assert.match(paper,/MAX_NOTIONAL_CENTS:\s*u64\s*=\s*250_000/,
  "Every simulated order must be risk capped");
assert.doesNotMatch(paper,/reqwest|tokio_tungstenite|std::net|TcpStream|CredentialManager|sign_order|revolut_x_private/,
  "Paper sandbox must not contain a network, credential or broker path");
assert.match(ipc,/fn accept_token\(&self, token: String, paper: &PaperSandbox\)/,
  "Replacing the authenticated session must own the paper reset");
assert.match(ipc,/let mut current = self\.token\(\);[\s\S]*?paper\.reset\(\);[\s\S]*?\*current = Some\(token\);/,
  "Session replacement and paper reset must hold the same Rust lock");
assert.match(ipc,/session\.accept_token\(token, &paper\);/,
  "Authenticated login and setup must reset the portfolio atomically");
assert.match(ipc,/fn paper_session_guard/,
  "Native paper commands must hold Rust auth lock during mutations");
for(const cmd of ["ipc_paper_snapshot","ipc_paper_trade","ipc_paper_reset"]) {
  const start=ipc.indexOf("fn "+cmd+"(");
  const end=ipc.indexOf("\n#[tauri::command]",start+1);
  const part=ipc.slice(start,end>start?end:ipc.length);
  assert.match(part,/paper_session_guard\(&session, window\.label\(\)\)/,
    "Paper commands must be restricted to the main authenticated session: "+cmd);
}

assert.match(html,/id="revolut-consent"[^>]*disabled/,"Optional network module must default disabled");
assert.match(js,/ipc_revolut_consent_get/,"UI must read native persisted consent");
assert.match(js,/ipc_revolut_consent_set/,"UI must persist opt-in through Rust IPC");
assert.match(ipc,/source == "revolut" && !consent\.enabled\(\)/,"Revolut feed must be gated in Rust");
assert.match(js,/window\.pravdaLockRevolutConsent\s*=\s*lockRevolutConsentUi/,
  "Lock must invalidate native feed consent UI after logout");
assert.match(js,/window\.refreshRevolutConsent\s*=\s*readRevolutConsent/,
  "Re-authentication must refresh native feed consent");
assert.match(js,/version !== revolutConsentVersion \|\| shell\.hidden/,
  "In-flight consent results must be ignored after logout");
assert.match(auth,/window\.pravdaLockRevolutConsent\(\)/,
  "Login screen must immediately lock stale optional-network controls");
assert.match(auth,/window\.refreshRevolutConsent\(\)/,
  "Signing back in must re-read permission from native Rust");
assert.doesNotMatch(js,/setInterval\(refreshSecurity(?:Events)?\s*,/,
  "No background SQLite/security diagnostic polling");
assert.match(js,/if \(!securityVisible\(\) \|\| securityCheckInFlight\) return;/,
  "Security scan must be visibility-gated and single-flight");
assert.match(js,/if \(tab === "security"\)/,
  "Opening Security must refresh its local status");

assert.match(html,/id="security-events"/,"Security events must be visible after login");
assert.match(ipc,/protonvpn.*https:\/\/protonvpn\.com\/support\//,"Proton documentation must be a fixed official URL");
assert.match(js,/renderAlerts\(Array\.isArray\(payload\.alerts\)/,
  "Simulator alerts must be rendered from the Rust IPC snapshot");
assert.match(js,/payload\.status==="unavailable" \|\| !Array\.isArray\(payload\.articles\)/,
  "Unavailable news must not be treated as a successful empty result");
assert.doesNotMatch(auth,/\bfetch\s*\(|https?:\/\/127\.0\.0\.1/);
const handlers=[...main.matchAll(/ipc::(ipc_[a-z_]+)/g)].map(x=>x[1]);
assert.equal(new Set(handlers).size,22);
for(const cmd of handlers) {
  assert.ok(build.includes('"'+cmd+'"'),"Missing manifest "+cmd);
  assert.ok(cap.permissions.includes("allow-"+cmd.replaceAll("_","-")),"Missing ACL "+cmd);
  assert.ok(ipc.includes("fn "+cmd+"("),"Missing handler "+cmd);
}
const asyncSessionBound = new Set([
  "ipc_market_snapshot","ipc_history_recent","ipc_history_query",
  "ipc_history_export_csv","ipc_paper_alerts_snapshot","ipc_fx_status",
  "ipc_public_feed","ipc_security_report","ipc_acceptance_report"
]);
for(const cmd of ["ipc_market_snapshot","ipc_history_recent","ipc_history_query",
 "ipc_history_export_csv","ipc_paper_alerts_snapshot","ipc_runtime_metrics",
 "ipc_fx_status","ipc_public_feed","ipc_revolut_consent_get","ipc_revolut_consent_set","ipc_open_source","ipc_session_check","ipc_security_report","ipc_acceptance_report","ipc_security_events","ipc_auth_logout"]) {
  const i=ipc.indexOf("fn "+cmd+"(");
  const next=ipc.indexOf("\n#[tauri::command]",i+1);
  const part=ipc.slice(i,next>i?next:ipc.length);
  if (asyncSessionBound.has(cmd)) {
    assert.match(part,/authorized_read_token\(&session, window\.label\(\)\)/,
      "Async IPC must capture authenticated session "+cmd);
    assert.match(part,/finish_authorized_read\(\s*&session,\s*&issued_to/,
      "Async IPC must reject result after logout or replacement "+cmd);
  } else {
    assert.match(part,/require_authenticated\(/,"Missing Rust auth guard "+cmd);
  }
}
assert.match(ipc,/current\.as_deref\(\) != Some\(issued_to\) \|\| !session\.authority\.session_valid\(issued_to\)/,
  "Async response barrier must bind original token and verify it is still valid");
// Exercise the real dashboard IPC adapter (not a copied mock) across
// logout/relogin. Old rejections must not revoke a newer Rust-authenticated UI.
assert.match(auth, /window\.pravdaAuthGeneration = 0;/);
assert.equal((auth.match(/window\.pravdaAuthGeneration\+\+;/g)||[]).length, 2,
  "Both logout-screen and successful login transitions must invalidate old UI requests");
const fetchStart = js.indexOf("    async function fetchJson(url) {");
const fetchEnd = js.indexOf("    async function refreshMarket()", fetchStart);
assert.ok(fetchStart >= 0 && fetchEnd > fetchStart, "Actual IPC adapter must be testable");
let release = null;
let relocks = 0;
const ui = {
  URL,
  window: {
    pravdaAuthGeneration: 2,
    pravdaRequireLogin() { relocks++; },
    pravdaInvoke() {
      return new Promise((resolve,reject) => { release = {resolve,reject}; });
    }
  }
};
vm.createContext(ui);
vm.runInContext(js.slice(fetchStart, fetchEnd), ui, {filename:"desktop-fetchJson.js"});
const oldRejected = ui.fetchJson("/api/geopolitics");
ui.window.pravdaAuthGeneration = 4; // session A logout + B login
release.reject(new Error("UNAUTHORIZED"));
await assert.rejects(oldRejected, /STALE_SESSION/);
assert.equal(relocks, 0, "Old session rejection must not hide new dashboard");

const oldSucceeded = ui.fetchJson("/api/market");
ui.window.pravdaAuthGeneration = 6;
release.resolve({value:"stale"});
await assert.rejects(oldSucceeded, /STALE_SESSION/);
assert.equal(relocks, 0, "Old successful data must not change new login state");

const currentDenied = ui.fetchJson("/api/market");
release.reject(new Error("UNAUTHORIZED"));
await assert.rejects(currentDenied, /UNAUTHORIZED/);
assert.equal(relocks, 1, "Rejection of current session must still show login");

const currentAllowed = ui.fetchJson("/api/market");
release.resolve({value:"current"});
assert.equal((await currentAllowed).value, "current",
  "Current session data must remain usable");

// Every protected asynchronous renderer must ignore a result (or a catch
// handler) from an older dashboard generation before mutating the new UI.
for (const name of [
  "refreshHistory","refreshAlerts","checkPriceAlerts","refreshSources",
  "refreshMarket","refreshRevolut","refreshSystem","refreshSecurityEvents",
  "refreshSecurity","runAcceptance","refreshFx","refreshFlows","refreshNews",
  "refreshCrypto","refreshAIS"
]) {
  const start = js.indexOf("    async function "+name+"(");
  const end = js.indexOf("\n    }", start);
  assert.ok(start >= 0 && end > start, "Missing UI refresh function "+name);
  const body = js.slice(start,end+6);
  assert.match(body,/const requestGeneration = window\.pravdaAuthGeneration;/,
    "UI refresh must identify initial authenticated generation: "+name);
  assert.match(body,/if \(requestGeneration !== window\.pravdaAuthGeneration\) return;/,
    "Stale UI refresh must not mutate the new login's dashboard: "+name);
}
const nodes = new Map();
ui.$ = id => {
  if (!nodes.has(id)) nodes.set(id,{value:"",textContent:"",href:"",
    replaceChildren(){this.textContent="";}});
  return nodes.get(id);
};
ui.t = key => key;
ui.setStatus = (target,_status,message) => {target.textContent=message;};
ui.renderFx = payload => {ui.$("fx-rates").textContent=payload.identity;};
ui.URLSearchParams = URLSearchParams;
ui.drawPriceChart = () => {};
ui.formatDate = String;
vm.runInContext("let historicalPoints=[];",ui);
function actualFrontend(name,next) {
  const start = js.indexOf("    async function "+name+"(");
  const end = js.indexOf("    async function "+next+"(",start+1);
  assert.ok(start>=0 && end>start,"Cannot isolate actual "+name+" implementation");
  vm.runInContext(js.slice(start,end),ui,{filename:"desktop-"+name+".js"});
}
actualFrontend("refreshFx","refreshFlows");
actualFrontend("refreshHistory","refreshAlerts");
actualFrontend("refreshNews","refreshCrypto");

// The new authenticated session B must win even if A completes *later*.
const oldFx = ui.refreshFx();
const releaseOldFx = release;
ui.window.pravdaAuthGeneration += 2;
const newFx = ui.refreshFx();
release.resolve({identity:"session-B"});
await newFx;
assert.equal(ui.$("fx-rates").textContent,"session-B");
releaseOldFx.reject(new Error("UNAUTHORIZED"));
await oldFx;
assert.equal(ui.$("fx-rates").textContent,"session-B",
  "Old FX rejection must not overwrite a newer login's FX values");
assert.equal(relocks,1,"Old FX rejection must not force login");

for (const [name,url,target,good,bad,payload] of [
  ["refreshHistory","/api/market/history","history-status","historyReady","unavailableFeed",
   {observations:[{ticks:1}]}],
  ["refreshNews","/api/geopolitics","news-list","noArticles","newsFailure",
   {status:"delayed",source:"GDELT",retrieved_at:"2026-11-04",articles:[]}]
]) {
  const old = ui[name]();
  const releaseOld = release;
  ui.window.pravdaAuthGeneration += 2;
  const newer = ui[name]();
  release.resolve(payload);
  await newer;
  const established = ui.$(target).textContent;
  assert.notEqual(established,bad,"B should show real "+name+" data");
  releaseOld.reject(new Error("UNAUTHORIZED"));
  await old;
  assert.equal(ui.$(target).textContent,established,
    "Late "+name+" rejection must preserve B's UI");
}
assert.equal(relocks,1,
  "No stale refresh completion may revoke or visually lock the active session");

console.log("Bundled desktop IPC-only frontend/ACL/CSP contracts OK");
