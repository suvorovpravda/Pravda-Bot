// Contract checks for IPC-only desktop. Native Windows E2E remains required.
import {readFileSync} from "node:fs";
import {join} from "node:path";
import assert from "node:assert/strict";
const root=join(process.cwd(),"crates/pravda-app");
const read=p=>readFileSync(join(root,p),"utf8");
const cfg=JSON.parse(read("tauri.conf.json"));
const cap=JSON.parse(read("capabilities/main.json"));
const main=read("src/main.rs"), ipc=read("src/ipc.rs"), build=read("build.rs");
const html=read("frontend/index.html"), js=read("frontend/dashboard.js"),auth=read("frontend/auth.js"), css=read("frontend/styles.css");
const installerWorkflow=readFileSync(join(process.cwd(),".github/workflows/windows-package.yml"),"utf8");
assert.equal(cfg.build.frontendDist,"frontend");
assert.deepEqual([...cfg.bundle.targets].sort(),["msi","nsis"],
  "Both MSI and NSIS must be produced for Windows release");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.type,"offlineInstaller",
  "Windows installers must contain the official offline WebView2 prerequisite");
assert.equal(cfg.bundle.windows?.webviewInstallMode?.silent,true,
  "WebView2 must install automatically, without user-run commands");
assert.match(installerWorkflow,/Smoke-test MSI installation and uninstall on pristine Windows/);
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
assert.match(js,/ipc_security_report/,"Security UI must retrieve an authenticated Rust-owned report");
assert.match(js,/ipc_security_events/,"Security events must remain authenticated Tauri IPC only");
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
assert.equal(new Set(handlers).size,18);
for(const cmd of handlers) {
  assert.ok(build.includes('"'+cmd+'"'),"Missing manifest "+cmd);
  assert.ok(cap.permissions.includes("allow-"+cmd.replaceAll("_","-")),"Missing ACL "+cmd);
  assert.ok(ipc.includes("fn "+cmd+"("),"Missing handler "+cmd);
}
for(const cmd of ["ipc_market_snapshot","ipc_history_recent","ipc_history_query",
 "ipc_history_export_csv","ipc_paper_alerts_snapshot","ipc_runtime_metrics",
 "ipc_fx_status","ipc_public_feed","ipc_revolut_consent_get","ipc_revolut_consent_set","ipc_open_source","ipc_session_check","ipc_security_report","ipc_security_events","ipc_auth_logout"]) {
  const i=ipc.indexOf("fn "+cmd+"(");
  const next=ipc.indexOf("\n#[tauri::command]",i+1);
  const part=ipc.slice(i,next>i?next:ipc.length);
  assert.match(part,/require_authenticated\(/,"Missing Rust auth guard "+cmd);
}
console.log("Bundled desktop IPC-only frontend/ACL/CSP contracts OK");
