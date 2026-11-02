#!/usr/bin/env node
// PRAVDA-BOT: automated NEGATIVE secret boundary regression gate (#74).
// This is an indicator-based audit, not a substitute for manual credential
// rotation or an independent security audit. Never echo matched credentials.
import assert from "node:assert/strict";
import {execFileSync} from "node:child_process";
import {readFileSync, existsSync, mkdtempSync, rmSync, writeFileSync, unlinkSync} from "node:fs";
import {tmpdir} from "node:os";
import {join} from "node:path";

function git(...args) {
  return execFileSync("git",args,{encoding:"utf8",maxBuffer:128*1024*1024,
    timeout:120000,windowsHide:true});
}
const files=git("ls-files","-z").split("\0").filter(Boolean);
const active=files.filter(p=>!p.startsWith("archive/"));
const checked=active.filter(p=>/\.(?:rs|js|mjs|ts|html|json|ya?ml|toml|md|ps1|cmd|bat|txt)$/i.test(p));
const sensitivePath=/(?:^|\/)(?:\.env(?:\..+)?|[^/]+\.(?:pem|key|p12|pfx|kdbx)|id_(?:rsa|ed25519))$/i;
const exceptions=[".env.example",".env.sample"];
for(const path of active) {
  assert.ok(!sensitivePath.test(path)||exceptions.includes(path),
    "Tracked private credential material path detected (name withheld)");
}
const signatures=[
  // Only high-confidence credential material patterns; cannot find all keys.
  /-----BEGIN (?:RSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY-----/,
  /(?:ghp_|gho_|ghu_|ghs_)[A-Za-z0-9]{36}/,
  /github_pat_[A-Za-z0-9_]{60,}/,
  /AKIA[0-9A-Z]{16}/,
  /sk_live_[A-Za-z0-9]{24,}/
];
function inspect(label,content) {
  for(const signature of signatures) {
    assert.ok(!signature.test(content), "Sensitive credential signature found in "+label+
      "; do not print the matching value; revoke and rotate immediately");
  }
}
for(const path of checked) inspect("active tracked source "+path,readFileSync(path,"utf8"));
const app=active.filter(p=>p.startsWith("crates/pravda-app/src/")||
  /^crates\/pravda-app\/frontend\/.*\.(?:js|html)$/i.test(p)||
  p.startsWith("crates/pravda-store/src/"));
for(const path of app) {
  const text=readFileSync(path,"utf8");
  assert.doesNotMatch(text,/\b(?:SecretStore|WindowsCredentialStore|SecretValue)\b/,
    "Secret service must never be exposed by frontend, Tauri handlers or DB");
}
assert.ok(existsSync("rust/pravda-server/src/secret_store.rs"),
  "Windows Credential Manager secret boundary must exist");
// Fetch-depth=0 in CI is a prerequisite: a shallow checkout is not evidence.
assert.equal(git("rev-parse","--is-shallow-repository").trim(),"false",
  "History secret audit requires full checkout; refuse shallow or partial history");
// Check all Git patch history for recognizable HIGH-CONFIDENCE key formats.
// This does NOT prove the absence of other secret formats; manual audit stays required.
// Regression: a merge-only credential must be detectable even if removed
// afterwards. Git omits merge commit patches without --diff-merges.
function verifyMergePatchDetection() {
  const temp=mkdtempSync(join(tmpdir(),"pravda-history-audit-"));
  function run(...args) {
    return execFileSync("git",args,{cwd:temp,encoding:"utf8",timeout:15000,windowsHide:true});
  }
  function commit(message) {
    run("-c","user.name=Pravda CI","-c","user.email=ci@invalid.local","commit","-m",message);
  }
  try {
    run("init","-q","-b","main");
    run("config","user.name","Pravda CI");
    run("config","user.email","ci@invalid.local");
    writeFileSync(join(temp,"base.txt"),"base\n");
    run("add","."); commit("base");
    run("checkout","-q","-b","topic");
    writeFileSync(join(temp,"topic.txt"),"feature\n");
    run("add","."); commit("topic");
    run("checkout","-q","main");
    writeFileSync(join(temp,"main.txt"),"main\n");
    run("add","."); commit("main");
    run("merge","--no-ff","--no-commit","topic");
    // Construct a harmless synthetic GitHub-style marker at runtime; never
    // commit a literal test credential to this production repository.
    const sentinel="ghp_"+"A".repeat(36);
    writeFileSync(join(temp,"merge-only.txt"),sentinel+"\n");
    run("add","."); commit("merge with deliberate synthetic marker");
    run("checkout","-q","-b","cleanup");
    writeFileSync(join(temp,"cleanup.txt"),"cleanup branch\\n");
    run("add","."); commit("cleanup branch");
    run("checkout","-q","main");
    writeFileSync(join(temp,"other.txt"),"another branch\\n");
    run("add","."); commit("other branch");
    run("merge","--no-ff","--no-commit","cleanup");
    unlinkSync(join(temp,"merge-only.txt"));
    run("add","-A"); commit("merge resolution scrubs synthetic marker");
    const plain=run("log","--all","--format=","--patch","--no-ext-diff");
    const complete=run("log","--all","--format=","--patch","--diff-merges=separate","--no-ext-diff");
    assert.equal(plain.includes(sentinel),false,
      "Fixture invalid: default history unexpectedly contains merge diff");
    assert.equal(complete.includes(sentinel),true,
      "Merge-only synthetic secret was missed by full-history scan");
  } finally {
    rmSync(temp,{recursive:true,force:true});
  }
}
verifyMergePatchDetection();
const patch=git("log","--all","--format=","--patch","--diff-merges=separate","--no-ext-diff");
inspect("historical Git patch (all fetched refs)",patch);
console.log("Secret boundary: active source, IPC isolation and fetched Git patch "+
  "high-confidence signatures OK (no credential values printed).");
