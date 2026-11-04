#!/usr/bin/env node
/**
 * CI contract: exactly three documentation pages, valid relative links,
 * synchronized, objective prioritized milestone percentages.
 */
import assert from "node:assert/strict";
import {execFileSync} from "node:child_process";
import {existsSync, readFileSync} from "node:fs";
import {dirname, resolve} from "node:path";

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
