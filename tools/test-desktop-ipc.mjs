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
assert.match(main,/#!\[cfg_attr\(windows, windows_subsystem = "windows"\)\]/,
  "Windows desktop must be a GUI PE; a console-bound GUI is killed when its terminal closes");
const desktop=readFileSync(join(process.cwd(),"rust/pravda-server/src/desktop.rs"),"utf8");
const binance=readFileSync(join(process.cwd(),"crates/pravda-fetchers/src/binance.rs"),"utf8");
const kraken=readFileSync(join(process.cwd(),"crates/pravda-fetchers/src/kraken.rs"),"utf8");
assert.match(desktop,/config\.market_feeds_enabled = false;/,
  "Diagnostic offline mode must force-disable WebSocket producers");
assert.match(desktop,/stream_state.*?as_object_mut/s,
  "A real market snapshot must expose the Rust feed status");
assert.match(js,/streamStatusMessage\(payload\.stream_state\)/,
  "UI must distinguish configured-offline feeds from waiting for first quote");
assert.match(binance,/wss:\/\/stream\.binance\.com:443\/ws/,
  "Spot market WSS must use the provider's documented 443 endpoint");
for(const [exchange,source] of [["Binance",binance],["Kraken",kraken]]) {
  assert.match(source,/timeout\(Duration::from_secs\(15\), connect_async/,
    exchange+" must bound TLS/WebSocket handshakes");
  assert.match(source,/timeout\(Duration::from_secs\(60\), read\.next\(\)\)/,
    exchange+" must not hang forever on silent WebSocket");
  assert.match(source,/Message::Pong\(payload\)/,
    exchange+" must respond to server ping");
}
assert.equal(cfg.build.frontendDist,"frontend");
assert.deepEqual([...cfg.bundle.targets].sort((a,b)=>a.localeCompare(b)),["msi","nsis"],
  "Both MSI and NSIS must be produced for Windows release");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.type,"offlineInstaller",
  "Windows installers must contain the official offline WebView2 prerequisite");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.silent,true,
  "WebView2 must install automatically, without user-run commands");
assert.match(installerWorkflow,/Smoke-test MSI installation and uninstall on pristine Windows/);
assert.match(installerWorkflow,/pravda-windows-single-file-setup/,
  "Single user-facing setup must be published independently from CI artifacts");
assert.match(setupSource,/GetSystemDirectoryW/,
  "Installer must resolve System32 using the native Windows API");
assert.match(setupSource,/fn trusted_windows_installer\(\)/,
  "Installer path must be validated before process launch");
assert.doesNotMatch(setupSource,/std::env::var_os\("SystemRoot"\)/,
  "Installer must not trust a caller-controlled SystemRoot environment variable");
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
assert.match(html,/class="terminal-cockpit"/,
  "Terminal must group real market, risk and watchlist modules in one cockpit");
assert.match(html,/id="terminal-watchlist"/,
  "Market watchlist must have a real data container");
assert.match(html,/data-watch-symbol="BTC\/USDT"/,
  "Watchlist must preserve the actual supported market symbols");
assert.match(js,/updateTerminalWatchlist\(rows\);/,
  "Watchlist must reuse the observed market snapshot, not a synthetic feed");
assert.match(js,/button\.dataset\.freshness = state;/,
  "Watchlist must label actual source freshness instead of claiming all ticks are live");
assert.match(js,/document\.querySelector\('\.tab-btn\[data-tab="markets"\]'\)\?\.click\(\)/,
  "Clicking an asset must open its existing market chart");
assert.equal((auth.match(/window\.pravdaClearMarketSession\(\)/g)||[]).length, 2,
  "Both logout and reauthentication must clear the previous market session");
assert.match(html,/class="grid terminal-market"/,
  "Market charts must remain a functional central workspace");
assert.match(html,/class="grid terminal-extras"/,
  "Security and runtime modules must remain visible in the same application");
assert.match(css,/grid-template-areas:\s*"watch market risk"/,
  "Large screens must use a three-pane institutional terminal grid");
assert.match(css,/grid-template-areas:\s*"market"\s*"watch"\s*"risk"/,
  "Narrow screens must prioritize the live chart");
assert.match(js,/document\.querySelector\("\.shell"\)\.dataset\.activeTab = tab;/,
  "Filtered navigation must expand its active functional module");
assert.match(js,/requestAnimationFrame\(drawPriceChart\)/,
  "Chart must redraw after switching from a hidden tab into the central workspace");
assert.match(css,/prefers-reduced-motion:\s*reduce/,
  "Visual transitions must respect system accessibility settings");
assert.match(html,/id="acceptance-center"/,
  "Installed desktop must own a native test center");
assert.match(html,/id="acceptance-pre"/, "Preproduction button must exist");
assert.match(html,/id="acceptance-post"/, "Postproduction button must exist");
assert.doesNotMatch(ipc,/phase\|verified\|/,
  "Report phase is metadata, not a successful test");
assert.match(js,/expectedAcceptanceChecks\[index\]/,
  "Acceptance rows must be ordered and unique before counting PASS");
assert.match(js,/run_number: runNumber/,
  "Each test press must carry an individual execution number");
assert.match(js,/checked_at_utc: finishedAt\.toISOString\(\)/,
  "Every completed test must publish its actual completion timestamp");
assert.match(js,/window\.pravdaDashboardReady = true;/,
  "Dashboard must advertise readiness only after all handlers are bound");
const bootDiagnosticStart=js.indexOf('window.pravdaDashboardPhase = "P02H";');
const bootDiagnosticEnd=js.indexOf('window.pravdaDashboardPhase = "P02I";',bootDiagnosticStart);
assert.ok(bootDiagnosticStart>=0 && bootDiagnosticEnd>bootDiagnosticStart,
  "First-use boot must still have bounded security initialization phase markers");
const bootDiagnosticScript=js.slice(bootDiagnosticStart,bootDiagnosticEnd);
// Static boot contract: do not execute arbitrary dashboard source in Node tests.
// The separate native WebView2 E2E suite exercises the actual runtime.
assert.doesNotMatch(bootDiagnosticScript,/\brefreshSecurity(?:Events)?\s*\(/,
  "First-use dashboard must not invoke optional protected diagnostics during bootstrap");
assert.doesNotMatch(bootDiagnosticScript,/\b(?:invoke|ipcCall)\s*\(/,
  "The diagnostic bootstrap must not initiate an IPC request");
assert.match(js,/if \(tab === "security"\) \{[\s\S]*?refreshSecurity\(true\);[\s\S]*?refreshSecurityEvents\(\);/,
  "Security panel must retain user-initiated diagnostics");
assert.match(js, /\$\("security-refresh"\)\.addEventListener\("click"/,
  "Manual security refresh button must remain connected");
const paperBootStart=js.indexOf('window.pravdaDashboardPhase = "P03";');
const paperBootEnd=js.indexOf('window.pravdaDashboardPhase = "P03B";',paperBootStart);
assert.ok(paperBootStart>=0 && paperBootEnd>paperBootStart,
  "P03 paper initialization must be bounded by exact phase markers");
const paperBootScript=js.slice(paperBootStart,paperBootEnd+
  'window.pravdaDashboardPhase = "P03B";'.length);
const firstPaperHandler=paperBootScript.indexOf('$("paper-order-form").addEventListener');
assert.ok(firstPaperHandler>0,"Paper form must bind after its initialization marker");
assert.doesNotMatch(paperBootScript.slice(0,firstPaperHandler),/\b(?:refreshPaper|paperAction)\s*\(/,
  "First-use bootstrap must not initiate a protected paper portfolio IPC");
assert.match(paperBootScript,/\$\("paper-order-form"\)\.addEventListener\("submit"/,
  "Paper form submission must be handled only through its event listener");
assert.match(paperBootScript,/\$\("paper-reset"\)\.addEventListener\("click"/,
  "Paper reset must remain a user-initiated event");
assert.match(paperBootScript,/window\.refreshPaper\s*=\s*\(\)\s*=>\s*refreshPaper\(true\)/,
  "Paper snapshot must remain manually callable after successful onboarding");
assert.match(js,/if \(tab === "security"\) \{[\s\S]*?refreshPaper\(true\);/,
  "Authenticated security tab must still fetch paper state on demand");
assert.match(auth,/if \(window\.pravdaDashboardReady !== true\)/,
  "A failed dashboard initialization must produce a visible local error");
assert.doesNotMatch(auth,/aucun mot de passe ni détail système n'a été enregistré/,
  "A successful first-use password setup may already be persisted before the dashboard fails");
assert.match(auth,/showDashboardStartupFailure\(dashboardFaultCode\)/,
  "A failed initialization must show a bounded diagnostic rather than leave a partial UI active");
assert.match(auth,/if \(dashboardLoading\) return;/,
  "Concurrent login callbacks must not inject dashboard.js twice");
assert.match(auth,/window\.pravdaDashboardReady = false;/,
  "Each load must clear stale dashboard readiness before starting a new script");
assert.match(auth,/dashboardLoading = false;/,
  "The dashboard script-load guard must release on completion or failure");
const diagnosticStart=auth.indexOf("// Capture only a closed diagnostic category");
const diagnosticEnd=auth.indexOf("// UI-only epoch",diagnosticStart);
assert.ok(diagnosticStart>=0 && diagnosticEnd>diagnosticStart,
  "Startup diagnostic handler must be extracted from the real auth script");
const bootNodes=[];
let bootWindowError;
const bootUi={
  window:{addEventListener(type,callback){if(type==="error") bootWindowError=callback;}},
  shell:{hidden:false},screen:{hidden:true,querySelector(){return null;}},
  form:{hidden:false},notice:{textContent:""},
  card:{append(node){bootNodes.push(node);}},
  document:{createElement(tag){return {
    tag,children:[],value:"",textContent:"",hidden:false,readOnly:false,
    setAttribute(){},addEventListener(){},append(...nodes){this.children.push(...nodes);}
  };}}
};
vm.createContext(bootUi);
vm.runInContext(auth.slice(diagnosticStart,diagnosticEnd),bootUi);
assert.equal(typeof bootWindowError,"function",
  "Authentication must observe dashboard runtime errors without collecting raw exceptions");
bootWindowError({
  filename:"https://tauri.localhost/dashboard.js",
  lineno:1234,
  error:{name:"TypeError",message:"SECRET_DO_NOT_LOG"}
});
vm.runInContext("showDashboardStartupFailure(dashboardFaultCode)",bootUi);
assert.equal(bootUi.shell.hidden,true,
  "An incompletely initialized dashboard must not remain interactive");
assert.equal(bootUi.screen.hidden,false);
assert.equal(bootUi.form.hidden,true,
  "Failed startup must not ask for the password again");
assert.equal(bootNodes.length,1);
const fields=bootNodes[0].children;
const diagnostic=fields.find(node=>node.tag==="input");
assert.equal(diagnostic?.value,"PRAVDA-UI-STARTUP|DASHBOARD_TYPE_L1234");
const bootPhase = fields.find(node=>node.tag==="input" && node !== diagnostic);
assert.equal(bootPhase?.value,"PRAVDA-UI-PHASE|PXX",
  "No user data or exception stack may appear in the bounded bootstrap phase");
const causeField = fields.find(node=>node.tag==="input" && node !== diagnostic && node !== bootPhase);
assert.equal(causeField?.value,"PRAVDA-UI-CAUSE|NOT_RANGE");
bootUi.window.pravdaDashboardPhase = "P02C";
bootWindowError({filename:"https://tauri.localhost/dashboard.js",lineno:1,
  error:{name:"RangeError",message:"Maximum call stack size exceeded - SECRET_DO_NOT_LOG"}});
vm.runInContext("showDashboardStartupFailure(dashboardFaultCode)",bootUi);
const refined=bootNodes.at(-1).children.filter(n=>n.tag==="input");
assert.equal(refined[0].value,"PRAVDA-UI-STARTUP|DASHBOARD_RANGE_L1");
assert.equal(refined[1].value,"PRAVDA-UI-PHASE|P02C");
assert.equal(refined[2].value,"PRAVDA-UI-CAUSE|STACK_OVERFLOW");
assert.doesNotMatch(JSON.stringify(refined),/SECRET_DO_NOT_LOG|tauri\.localhost/,
  "Only an allowlisted static cause and substage may be printed for native failures");
assert.equal((js.match(/window\.pravdaDashboardPhase = "P02[A-I]";/g)||[]).length,9,
  "RangeError diagnostic must isolate each early bootstrap stage");
assert.doesNotMatch(js,/Math\.(?:min|max)\(\.\.\./,
  "Large public market, energy or AIS observations must not be spread into Math calls");
const extentStart=js.indexOf("    function finiteExtent(values) {");
const extentEnd=js.indexOf("    function drawPriceChart()",extentStart);
assert.ok(extentStart>=0 && extentEnd>extentStart);
const extent = vm.runInNewContext(js.slice(extentStart,extentEnd) + "\nfiniteExtent;", {});
const largeSeries=Array.from({length:250000},(_,i)=>i);
assert.equal(extent(largeSeries).min,0);
assert.equal(extent(largeSeries).max,249999);
assert.equal((js.match(/window\.pravdaDashboardPhase = "P[0-9]{2}";/g)||[]).length,16,
  "Installed GUI must expose static startup phases for regression diagnosis");
assert.doesNotMatch(JSON.stringify(fields),/SECRET_DO_NOT_LOG|tauri\.localhost/,
  "Diagnostics must never contain exception text, source URLs, stack traces or credentials");
assert.match(js,/button\.textContent = "↻ Tentative à "/,
  "Manual refresh must provide visible feedback without inventing fresh market data");
assert.match(js,/queuedAuthRefreshGeneration = requestedGeneration;/,
  "Reauthentication refresh must queue behind a prior in-flight session batch");
assert.match(js,/nextGeneration === window\.pravdaAuthGeneration/,
  "Queued refresh must be bound to the current authenticated Rust session");
assert.match(js,/void refreshAll\(\);/,
  "Queued refresh must execute once the old in-flight operation releases");
assert.match(js,/nextGeneration === window\.pravdaAuthGeneration &&\s*securityShell && !securityShell\.hidden/,
  "Queued refresh must use the existing main shell reference after reauthentication");
assert.doesNotMatch(js,/\$\("shell"\)/,
  "Queued refresh must not dereference an element ID absent from the desktop HTML");
assert.match(html,/les mêmes contrôles locaux Rust/,
  "Preproduction and postproduction must describe their current shared scope");
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
assert.match(js,/if \(generation !== window\.pravdaAuthGeneration\) void refreshPaper\(true\);/,
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
assert.match(js,/const securitySessionReady = \(\) => !!securityShell && !securityShell.hidden;/,
  "Authenticated UI session must own explicit test interactions");
assert.match(js,/if \(!explicit && !securityVisible\(\)\) return;/,
  "Only passive checks may be skipped for hidden WebView2 documents");
assert.match(js,/if \(securityCheckInFlight\) \{/,
  "Security tests must be single-flight");
assert.match(js,/window\.refreshSecurity = \(\) => refreshSecurity\(true\);/,
  "User-driven security checks must not be silently skipped because of visibilityState");
assert.doesNotMatch(js,/if \(!securityVisible\(\) \|\| acceptanceBusy/,
  "Manual acceptance controls must never silently ignore a click for document.hidden");
assert.match(js,/if \(tab === "security"\)/,
  "Opening Security must refresh its local status");

assert.match(html,/id="updater-center"/,"Signed updater readiness must appear inside authenticated Security settings");
assert.match(html,/id="updater-readiness"/,"User needs a local config status action, not a fake update installer");
assert.match(js,/window\.pravdaInvoke\("ipc_update_status"\)/,
  "Updater status must come from authenticated Rust IPC without direct network or shell permissions");
assert.match(js,/values\.automatic !== "disabled"/,
  "Unsigned builds must not be misrepresented as automatically updating");
assert.match(ipc,/fn update_status_rows\(\) -> Vec<String>/,
  "Updater status must be a closed, local-only Rust allowlist");
assert.match(ipc,/automatic\|disabled/,
  "Auto-updater must remain blocked until a genuine key and verified release exist");
assert.match(js,/if \(sessionChanged && securitySessionReady\(\)\) \{\s*void refreshUpdateReadiness\(\);/,
  "An old updater IPC completion must re-query only the current authenticated session");
assert.match(auth,/window\.pravdaClearUpdateReadiness\(\)/,
  "Authentication transitions must reset stale update status");
assert.match(html,/id="security-events"/,"Security events must be visible after login");
assert.match(ipc,/protonvpn.*https:\/\/protonvpn\.com\/support\//,"Proton documentation must be a fixed official URL");
assert.match(js,/renderAlerts\(Array\.isArray\(payload\.alerts\)/,
  "Simulator alerts must be rendered from the Rust IPC snapshot");
assert.match(js,/payload\.status==="unavailable" \|\| !Array\.isArray\(payload\.articles\)/,
  "Unavailable news must not be treated as a successful empty result");
assert.doesNotMatch(auth,/\bfetch\s*\(|https?:\/\/127\.0\.0\.1/);
const handlers=[...main.matchAll(/ipc::(ipc_[a-z_]+)/g)].map(x=>x[1]);
assert.equal(new Set(handlers).size,23);
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
 "ipc_history_export_csv","ipc_paper_alerts_snapshot","ipc_runtime_metrics","ipc_update_status",
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
vm.runInContext("let historicalPoints=[], fxRequestNumber=0, lastSettledFxRequest=0;",ui);
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

// Run the actual market-session clearing and language-change code. In
// particular, a language change must not replay an older session's quotes.
const marketNodes = new Map();
const watchRows = ["BTC/USDT", "ETH/USDT", "SOL/USDT"].map(symbol => {
  const parts = {".watch-price": {textContent:"session-A price"},
    ".watch-freshness": {textContent:"Récent"}};
  return {dataset:{watchSymbol:symbol,freshness:"live"},
    querySelector: selector => parts[selector]};
});
const marketUi = {
  window:{pravdaAuthGeneration:4},
  document:{documentElement:{lang:"fr"},title:"",
    querySelectorAll: selector => selector === ".watchlist-row" ? watchRows : []},
  $: id => {
    if (!marketNodes.has(id)) marketNodes.set(id,{value:"",textContent:"session-A",
      replaceChildren(){this.textContent="";}});
    return marketNodes.get(id);
  },
  t:key=>key,
  setStatus:(target,state,label)=>{target.state=state;target.textContent=label;},
  updateClock(){}, refreshAlerts(){}, refreshSources(){}, drawPriceChart(){},
  marketRenders:[], fxRenders:[],
  renderMarket(payload){this.marketRenders.push(payload);},
  renderFx(payload){this.fxRenders.push(payload);},
  fetchJson:async()=>{throw new Error("offline");}
};
// Function calls in the isolated VM are unbound; keep their observations in
// the shared fixture, rather than depending on `this`.
marketUi.renderMarket = payload => {
  marketUi.marketRenders.push(payload);
  marketUi.lastPayload = payload;
  vm.runInContext("latestMarket=lastPayload",marketUi);
};
marketUi.renderFx = payload => {
  marketUi.fxRenders.push(payload);
  marketUi.lastFxPayload = payload;
  vm.runInContext("latestFx=lastFxPayload",marketUi);
};
vm.createContext(marketUi);
vm.runInContext('let currentLanguage="fr", latestMarket={session:"A"}, ' +
  'latestFx={session:"A"}, historicalPoints=[{price:123}];',marketUi);
function runRealSection(startMarker,endMarker) {
  const start=js.indexOf(startMarker), end=js.indexOf(endMarker,start+1);
  assert.ok(start>=0 && end>start,"Missing actual frontend section: "+startMarker);
  vm.runInContext(js.slice(start,end),marketUi,{filename:"desktop-market-session.js"});
}
runRealSection("    function applyLanguage()", "    const $ =");
runRealSection("    function clearMarketSnapshot()", "    function updateTerminalWatchlist(");
runRealSection("    let marketRequestNumber = 0;", "    // Auth boundaries invalidate");
runRealSection("    let fxRequestNumber = 0;", "    async function refreshFlows()");
marketUi.window.pravdaClearMarketSession();
marketUi.applyLanguage();
assert.equal(marketUi.marketRenders.length,0,"Language change must not replay session A quotes");
assert.equal(marketUi.fxRenders.length,0,"Language change must not replay session A FX");
assert.equal(vm.runInContext("latestMarket",marketUi),null);
assert.equal(vm.runInContext("latestFx",marketUi),null);
assert.equal(vm.runInContext("historicalPoints.length",marketUi),0);
assert.equal(marketUi.$("chart-price").textContent,"—");
assert.equal(marketUi.$("crypto-quotes").textContent,"");
for (const row of watchRows) {
  assert.equal(row.dataset.freshness,"unavailable");
  assert.equal(row.querySelector(".watch-price").textContent,"—");
}
vm.runInContext('latestMarket={session:"B"}; latestFx={session:"B"};',marketUi);
marketUi.applyLanguage();
assert.equal(marketUi.marketRenders.at(-1).session,"B","Current session quotes remain usable");
assert.equal(marketUi.fxRenders.at(-1).session,"B","Current session FX remains usable");
await marketUi.refreshMarket();
const rendersAfterFailure=marketUi.marketRenders.length;
marketUi.applyLanguage();
assert.equal(marketUi.marketRenders.length,rendersAfterFailure,
  "Language change must not relabel a failed market refresh as live");
vm.runInContext('latestMarket={session:"market-still-healthy"}; latestFx={session:"B"};',marketUi);
const rendersBeforeFxFailure=marketUi.marketRenders.length;
await marketUi.refreshFx();
assert.equal(vm.runInContext("latestFx",marketUi),null,
  "A failed FX refresh must not leave old rates usable for conversions");
assert.equal(vm.runInContext("latestMarket.session",marketUi),"market-still-healthy",
  "An FX-only failure must preserve the independent market snapshot");
assert.equal(marketUi.marketRenders.length,rendersBeforeFxFailure+1,
  "An FX-only failure must redraw market observations without stale conversions");
const marketPending=[];
marketUi.fetchJson=()=>new Promise((resolve,reject)=>marketPending.push({resolve,reject}));
const oldMarket=marketUi.refreshMarket();
const newMarket=marketUi.refreshMarket();
marketPending[1].reject(new Error("offline"));
await newMarket;
const rendersBeforeOldSuccess=marketUi.marketRenders.length;
marketPending[0].resolve({session:"older-success"});
await oldMarket;
assert.equal(marketUi.marketRenders.length,rendersBeforeOldSuccess,
  "An older successful request must not undo a newer failed market refresh");
const oldFailure=marketUi.refreshMarket();
const newSuccess=marketUi.refreshMarket();
marketPending[3].resolve({session:"newest"});
await newSuccess;
marketPending[2].reject(new Error("offline"));
await oldFailure;
assert.equal(vm.runInContext("latestMarket.session",marketUi),"newest",
  "An older failure must not erase a newer successful market refresh");
const fxPending=[];
marketUi.fetchJson=()=>new Promise((resolve,reject)=>fxPending.push({resolve,reject}));
const oldFxRequest=marketUi.refreshFx();
const newFxRequest=marketUi.refreshFx();
fxPending[1].resolve({session:"newest-FX"});
await newFxRequest;
fxPending[0].reject(new Error("offline"));
await oldFxRequest;
assert.equal(vm.runInContext("latestFx.session",marketUi),"newest-FX",
  "An older FX failure must not erase a newer successful FX refresh");
const earlierFxSuccess=marketUi.refreshFx();
const laterFxFailure=marketUi.refreshFx();
fxPending[3].reject(new Error("offline"));
await laterFxFailure;
const fxRendersAfterFailure=marketUi.fxRenders.length;
fxPending[2].resolve({session:"obsolete-FX"});
await earlierFxSuccess;
assert.equal(marketUi.fxRenders.length,fxRendersAfterFailure,
  "An older FX success must not undo a newer failed FX refresh");
assert.equal(vm.runInContext("latestFx",marketUi),null);

// Execute the *actual* updater readiness UI closure with an old IPC response
// delayed across logout and reauthentication. The new login must win without
// displaying old-session data or requiring another manual click.
const updaterStart = js.indexOf("    let updateReadinessBusy = false;");
const updaterEnd = js.indexOf("    // Paper Trading is a deterministic",updaterStart);
assert.ok(updaterStart >= 0 && updaterEnd > updaterStart,
  "Updater readiness logic must remain extractable for a real deferred IPC regression");
const updaterNodes = new Map();
const updaterPending = [];
let updaterSessionReady = true;
const updaterVm = {
  window:{
    pravdaAuthGeneration: 1,
    pravdaInvoke: () => new Promise((resolve,reject) => {
      updaterPending.push({resolve,reject});
    })
  },
  securitySessionReady: () => updaterSessionReady,
  $: id => {
    if (!updaterNodes.has(id)) updaterNodes.set(id,{
      disabled:false, textContent:"",addEventListener() {}
    });
    return updaterNodes.get(id);
  }
};
vm.createContext(updaterVm);
vm.runInContext(js.slice(updaterStart,updaterEnd),updaterVm,{
  filename:"pravda-updater-readiness-session.js"
});
assert.equal(updaterPending.length,1,
  "Authenticated dashboard initially requests a local Rust updater state");
updaterSessionReady = false;
updaterVm.window.pravdaAuthGeneration++;
updaterVm.window.pravdaClearUpdateReadiness();
updaterSessionReady = true;
updaterVm.window.pravdaAuthGeneration++;
updaterVm.window.refreshUpdateReadiness();
assert.equal(updaterPending.length,1,
  "A new session does not create overlapping updater requests");
updaterPending[0].resolve([
  "version|0.3.4","channel|not_configured","signature|not_verified",
  "automatic|disabled","last_check|never"
]);
await new Promise(setImmediate);
assert.equal(updaterPending.length,2,
  "Old completion automatically rechecks the new session after releasing guard");
assert.ok(!updaterVm.$("updater-state").textContent.includes("Version 0.3.4"),
  "Old response must not leak its updater state to the new session");
updaterPending[1].resolve([
  "version|0.3.4","channel|not_configured","signature|not_verified",
  "automatic|disabled","last_check|never"
]);
await new Promise(setImmediate);
assert.match(updaterVm.$("updater-state").textContent,/Version 0\.3\.4/);
assert.match(updaterVm.$("updater-state").textContent,/non configurées/);
assert.equal(updaterVm.$("updater-readiness").disabled,false,
  "Updater configuration action must recover after stale IPC resolves");
assert.match(updaterVm.$("updater-checked").textContent,/aucun appel réseau/);

console.log("Bundled desktop IPC-only frontend/ACL/CSP contracts OK");
