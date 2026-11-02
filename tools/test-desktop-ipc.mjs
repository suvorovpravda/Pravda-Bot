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
assert.equal(cfg.build.frontendDist,"frontend");
assert.equal(cfg.app.withGlobalTauri,true);
assert.deepEqual(cfg.app.security.capabilities,["main-ipc"]);
assert.match(cfg.app.security.csp["connect-src"],/ipc:/);
assert.doesNotMatch(cfg.app.security.csp["script-src"],/unsafe-inline|https?:/);
assert.deepEqual(cap.windows,["main"]);
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
assert.doesNotMatch(auth,/\bfetch\s*\(|https?:\/\/127\.0\.0\.1/);
const handlers=[...main.matchAll(/ipc::(ipc_[a-z_]+)/g)].map(x=>x[1]);
assert.equal(new Set(handlers).size,12);
for(const cmd of handlers) {
  assert.ok(build.includes('"'+cmd+'"'),"Missing manifest "+cmd);
  assert.ok(cap.permissions.includes("allow-"+cmd.replaceAll("_","-")),"Missing ACL "+cmd);
  assert.ok(ipc.includes("fn "+cmd+"("),"Missing handler "+cmd);
}
for(const cmd of ["ipc_market_snapshot","ipc_history_recent","ipc_history_query",
 "ipc_history_export_csv","ipc_paper_alerts_snapshot","ipc_runtime_metrics",
 "ipc_fx_status","ipc_session_check","ipc_auth_logout"]) {
  const i=ipc.indexOf("fn "+cmd+"(");
  const next=ipc.indexOf("\n#[tauri::command]",i+1);
  const part=ipc.slice(i,next>i?next:ipc.length);
  assert.match(part,/require_authenticated\(/,"Missing Rust auth guard "+cmd);
}
console.log("Bundled desktop IPC-only frontend/ACL/CSP contracts OK");
