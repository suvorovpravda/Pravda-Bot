#!/usr/bin/env node
// GitHub Actions action supply-chain contract. Ref tags such as @v7 or
// @stable can be moved; SHA pinning protects the workflows we execute.
import assert from "node:assert/strict";
import {readdirSync, readFileSync} from "node:fs";

const trusted = new Map([
  ["actions/checkout","3d3c42e5aac5ba805825da76410c181273ba90b1"],
  ["actions/setup-node","949feb2413d6458794dcd2491c4babbbce0c15c1"],
  ["actions/cache","55cc8345863c7cc4c66a329aec7e433d2d1c52a9"],
  ["actions/upload-artifact","cf430e030ddbb5b0abf93d22962f4752f3646cd9"],
  ["actions/download-artifact","9000827ccba6bdab643e8b6fd33ac0654aef8333"],
  ["dtolnay/rust-toolchain","b725eda525362e7cf34af9a7243cac1e3f01a7ec"],
  ["github/codeql-action/init","33d7b85da1a04171e211befe2c739303b9a4c066"],
  ["github/codeql-action/analyze","33d7b85da1a04171e211befe2c739303b9a4c066"],
]);
// The Rust MSRV action uses a different trusted commit from "stable".
const msrvSha="035fceb3c12fb0c6a9461b52775e16473133e591";
const dir=".github/workflows";
let checked=0;
const errors=[];
for(const name of readdirSync(dir).filter(p=>/\.ya?ml$/i.test(p)).sort()) {
  const lines=readFileSync(dir+"/"+name,"utf8").split(/\r?\n/);
  for(const [index,line] of lines.entries()) {
    const match=line.match(/^\s*(?:-\s+)?uses:\s*([^\s#]+)/);
    if(!match)continue;
    const source=match[1];
    if(source.startsWith("./"))continue;
    const at=source.lastIndexOf("@");
    const action=source.slice(0,at), sha=source.slice(at+1);
    checked++;
    if(at<0 || !/^[a-f0-9]{40}$/i.test(sha)) {
      errors.push(name+":"+(index+1)+": remote action must use full commit SHA");
      continue;
    }
    const expected=trusted.get(action);
    if(!expected || (sha!==expected && !(action==="dtolnay/rust-toolchain" && sha===msrvSha)))
      errors.push(name+":"+(index+1)+": action or commit not in reviewed allowlist");
  }
}
assert.ok(checked>=30,"Expected to inspect all active Windows, Rust, security and docs workflows");
assert.deepEqual(errors,[],"All GitHub Actions must use reviewed, immutable commit SHAs");
// PR checks must prove the branch HEAD, not the synthetic refs/pull/*/merge SHA.
const exactCheckoutRef = "ref: ${{ github.event.pull_request.head.sha || github.sha }}";
for (const [workflow,required] of Object.entries({
  "rust.yml":7, "rustsec-audit.yml":2, "codeql.yml":1,
  "browser-fallback.yml":1, "windows-package.yml":2, "live-docs.yml":1
})) {
  const source=readFileSync(dir+"/"+workflow,"utf8");
  assert.equal(source.split(exactCheckoutRef).length-1,required,
    workflow+": PR checkouts must be pinned to the actual head");
  assert.equal(source.split("- name: Enforce exact checked-out source SHA").length-1,required,
    workflow+": exact SHA must be asserted before tests");
}
assert.match(readFileSync(dir+"/live-docs.yml","utf8"),/ref: main/,
  "Live status publisher must remain on protected main");
for (const workflow of ["windows-package.yml","rustsec-audit.yml"]) {
  assert.ok(!readFileSync(dir+"/"+workflow,"utf8").includes("${{ github.sha }}"),
    workflow+": artifact labels must not use the synthetic merge commit");
}
assert.match(readFileSync(dir+"/rust.yml","utf8"),
  /\$checkedOutSha = \(git rev-parse HEAD\)\.Trim\(\)/,
  "Native WebView2 evidence must match the checked-out source commit");
console.log("GitHub Actions supply-chain: "+checked+" remote action references pinned to reviewed SHA");
