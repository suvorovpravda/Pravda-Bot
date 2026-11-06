#!/usr/bin/env node
/**
 * CI contract: exactly three documentation pages, valid relative links,
 * synchronized, objective prioritized milestone percentages.
 */
import assert from "node:assert/strict";
import {execFileSync} from "node:child_process";
import {existsSync, readFileSync} from "node:fs";
import {dirname, resolve} from "node:path";
import {isIP} from "node:net";

const tracked=execFileSync("git",["ls-files","-z"],{encoding:"utf8"})
  .split("\0").filter(Boolean);
const expected=["README.md","SECURITY.md","docs/PRAVDA-GUIDE.md"];
const docs=tracked.filter(p=>p.endsWith(".md")).sort();
assert.deepEqual(docs,expected.slice().sort(),
  "Exactly three Markdown docs: README, SECURITY, central guide");

function progressRows(path) {
  const content=readFileSync(path,"utf8"), results=[];
  for(const [i,line] of content.split(/\r?\n/).entries()) {
    if(!/^\|\s*\*\*P[012]\*\*\s*\|/.test(line))continue;
    const fraction=line.match(/\b(\d+)\s*\/\s*(\d+)\b/);
    const percent=line.match(/\b(\d+)\s*%/);
    if(!fraction||!percent)continue;
    const done=Number(fraction[1]),total=Number(fraction[2]),value=Number(percent[1]);
    assert.ok(total>0&&done>=0&&done<=total,
      path+":"+(i+1)+" invalid milestones");
    assert.equal(value,Math.round(done*100/total),
      path+":"+(i+1)+" milestone percent is wrong");
    results.push({priority:line.match(/^\|\s*\*\*(P[012])\*\*/)[1],done,total,value});
  }
  assert.equal(results.length,9,path+": require all 9 P0/P1/P2 workstreams");
  for(let i=1;i<results.length;i++)
    assert.ok(results[i].priority>=results[i-1].priority,
      path+": tasks must be ordered P0, P1 then P2");
  return results;
}
const reference=progressRows("README.md");
for(const page of expected.slice(1))
  assert.deepEqual(progressRows(page),reference,
    page+": update progress in README, SECURITY and guide together");

for(const page of expected) {
  const markdown=readFileSync(page,"utf8");
  const links=[...markdown.matchAll(/\]\(([^)]+)\)/g)].map(x=>x[1]);
  for(const link of links) {
    if(/^(?:https?:\/\/|mailto:|#|app:\/\/)/i.test(link))continue;
    const target=decodeURIComponent(link.split("#")[0].split("?")[0]);
    if(!target)continue;
    assert.ok(existsSync(resolve(dirname(page),target)),
      page+": broken relative link "+link);
  }
  assert.doesNotMatch(markdown,
    /\]\((?:docs\/architecture\/|docs\/releases\/)[^)]+\.md(?:#[^)]*)?\)/,
    page+": historical page link not replaced");
}
// A release documentation update must reflect the actual Windows installer
// bootstrap mode instead of promising an unverified download/offline path.
const windowsConfig=JSON.parse(readFileSync("crates/pravda-app/tauri.conf.json","utf8"));
assert.equal(windowsConfig.bundle?.windows?.webviewInstallMode?.type,"offlineInstaller",
  "The Windows packaging contract requires an embedded WebView2 prerequisite");
assert.equal(windowsConfig.bundle?.windows?.webviewInstallMode?.silent,true);
// Signed updater is fail-closed until a release public key and a fixed HTTPS
// metadata channel are configured. This is a static trust-boundary check,
// NOT evidence that an updater or a signed release exists.
function validateUpdaterEndpoint(endpoint) {
  assert.equal(typeof endpoint,"string","Updater endpoint must be a string");
  assert.doesNotMatch(endpoint,/\{\{|\$\{/,"Updater endpoint must not contain substitutions");
  const url=new URL(endpoint);
  assert.equal(url.protocol,"https:","Updater metadata must use HTTPS");
  assert.equal(url.username,"","Updater metadata must not contain credentials");
  assert.equal(url.password,"","Updater metadata must not contain credentials");
  assert.equal(url.port,"","Updater metadata must use default HTTPS port");
  assert.equal(url.hash,"","Updater metadata must not contain a fragment");
  assert.equal(url.search,"","Updater metadata must not contain query parameters");
  assert.equal(url.hostname.toLowerCase(),"github.com",
    "Updater metadata must be hosted on the fixed GitHub release channel");
  assert.match(url.pathname,
    /^\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+\/releases\/latest\/download\/latest\.json$/,
    "Updater metadata must use the stable latest.json release asset path");
  const host=url.hostname.replace(/^\[|\]$/g,"").toLowerCase();
  assert.ok(host && !isIP(host) && host!=="localhost" &&
    !host.endsWith(".localhost") && !host.endsWith(".local") &&
    !host.endsWith(".internal") && !host.endsWith(".test") &&
    !host.endsWith(".example") && !host.endsWith(".invalid") &&
    !["example.com","example.org","example.net"].includes(host),
    "Updater endpoint must use a public DNS hostname, not a local address or placeholder");
}
for(const bad of [
  "https://[::1]/metadata.json",
  "https://127.1/metadata.json",
  "https://2130706433/metadata.json",
  "https://127.0.0.1/metadata.json",
  "https://localhost/metadata.json",
  "https://service.local/metadata.json",
  "https://user:pass@github.com/latest.json",
  "http://github.com/latest.json",
  "https://github.com:8443/latest.json",
  "https://github.com/owner/repo/releases/latest/download/latest.json?token=secret",
  "https://github.com/owner/repo/releases/latest/download/other.json",
  "https://github.com/owner/repo/releases/download/v1/latest.json",
  "https://attacker.example.net/owner/repo/releases/latest/download/latest.json",
  "https://example.com/latest.json",
  "https://github.com/{{version}}/latest.json"
]) assert.throws(()=>validateUpdaterEndpoint(bad),
  "Updater trust gate must reject unsafe endpoint: "+bad);
assert.doesNotThrow(()=>validateUpdaterEndpoint(
  "https://github.com/example/repository/releases/latest/download/latest.json"));
const updater=windowsConfig.plugins?.updater;
if(updater?.active===true) {
  assert.equal(typeof updater.pubkey,"string","Updater requires a public verification key");
  assert.ok(updater.pubkey.trim().length>40,"Updater public key must not be empty or a placeholder");
  assert.ok(Array.isArray(updater.endpoints)&&updater.endpoints.length===1,
    "Updater must use one fixed release metadata endpoint");
  validateUpdaterEndpoint(updater.endpoints[0]);
} else {
  assert.ok(updater?.active!==true,"Unsigned update channel must remain disabled");
}
for(const page of expected) {
  const content=readFileSync(page,"utf8");
  assert.match(content,/offlineInstaller/,
    page+": must describe the actual embedded WebView2 prerequisite");
  assert.doesNotMatch(content,/webviewInstallMode=downloadBootstrapper/,
    page+": contains the retired installer mode");
}
assert.match(readFileSync(".gitattributes","utf8"),
  /archive\/python-legacy\/\*\* linguist-vendored/);
console.log("Docs boundary OK: exactly three Markdown pages");
console.log("Relative links valid, all nine prioritized percentages consistent");
