#!/usr/bin/env node
/**
 * CI gate: production sources and build entry points must remain Rust/Tauri.
 * Python is retained as historic reference only under archive/python-legacy.
 * This verifies SOURCE SEPARATION, not 100% feature parity with old Python.
 */
import assert from "node:assert/strict";
import {execFileSync} from "node:child_process";
import {readFileSync} from "node:fs";

const tracked=execFileSync("git",["ls-files","-z"],{encoding:"utf8"})
  .split("\0").filter(Boolean);
const archive="archive/python-legacy/";
const active=tracked.filter(p=>!p.startsWith(archive));
const historical=tracked.filter(p=>p.startsWith(archive));
const problems=[];
function requireCheck(ok,description) {
  if(!ok) problems.push(description);
}
requireCheck(historical.filter(p=>p.endsWith(".py")).length>=50,
  "Expected historical Python sources under archive/python-legacy");
requireCheck(active.some(p=>p==="Cargo.toml"),"Root Rust workspace Cargo.toml missing");
requireCheck(active.some(p=>p==="crates/pravda-app/src/main.rs"),"Tauri Rust main missing");
requireCheck(active.some(p=>p==="crates/pravda-app/capabilities/main.json"),"Tauri ACL manifest missing");
const attr=readFileSync(".gitattributes","utf8");
requireCheck(attr.includes("archive/python-legacy/** linguist-vendored"),
  "Archive must be ignored by GitHub Linguist language statistics");

const packaging=/(^|\/)(?:pyproject\.toml|setup\.(?:py|cfg)|MANIFEST\.in|Pipfile(?:\.lock)?|poetry\.lock|uv\.lock|\.python-version|requirements[^/]*\.(?:in|txt))$/i;
for(const path of active) {
  if(path.endsWith(".py")) problems.push("Active Python source: "+path);
  if(packaging.test(path)) problems.push("Active Python packaging: "+path);
  if(/(^|\/)(?:__pycache__|[^/]+\.egg-info)(?:\/|$)/i.test(path))
    problems.push("Tracked Python build artifacts outside archive: "+path);
}
for(const path of active.filter(p=>p.endsWith("Cargo.toml"))) {
  const toml=readFileSync(path,"utf8");
  if(/^\s*(?:pyo3|maturin|cpython)\s*=/m.test(toml))
    problems.push("Active Cargo manifest enables Python FFI: "+path);
}

// Scripts/Actions that BUILD or LAUNCH the active application must not
// execute Python. Inspect executable statements, not archived or disabled CI.
const launchScripts=active.filter(p=>
  p==="run_server.bat" ||
  /^tools\/[^/]+\.(?:bat|cmd|ps1)$/i.test(p) ||
  /^\.github\/workflows\/[^/]+\.ya?ml$/i.test(p)
);
const interpreter=/(?:^|[\\s;&|])(?:python(?:3(?:\\.\\d+)?)?(?:\\.exe)?|py(?:\\.exe)?)\\b/i;
function executableStatements(path, lines) {
  if(!path.startsWith(".github/workflows/")) {
    return lines.map((text,index)=>({text:text.trim(),line:index+1}));
  }
  // GitHub Actions YAML: descriptive metadata (name, comments, env names)
  // must never count as a Python interpreter invocation. Only inspect
  // executable run commands and explicitly configured Python action/shells.
  const commands=[];
  let blockIndent=null;
  for(let index=0;index<lines.length;index++) {
    const raw=lines[index], trim=raw.trim();
    const indent=raw.length-raw.trimStart().length;
    const match=raw.match(/^\\s*(?:-\\s+)?run\\s*:\\s*(.*)$/);
    if(match) {
      blockIndent=null;
      const value=match[1].trim();
      if(/^[|>][-+]?\\s*(?:#.*)?$/.test(value)) {
        blockIndent=indent;
      } else if(value) {
        commands.push({text:value,line:index+1});
      }
      continue;
    }
    if(blockIndent!==null && indent>blockIndent) {
      commands.push({text:trim,line:index+1});
      continue;
    }
    blockIndent=null;
    if(/^\\s*(?:-\\s+)?uses\\s*:\\s*actions\\/setup-python@/i.test(raw)
      || /^\\s*shell\\s*:\\s*(?:python|py)(?:\\s|$)/i.test(raw)) {
      commands.push({text:"python",line:index+1});
    }
  }
  return commands;
}
for(const path of launchScripts) {
  const lines=readFileSync(path,"utf8").split(/\\r?\\n/);
  for(const {text:line,line:number} of executableStatements(path,lines)) {
    if(!line || /^(?:#|rem(?:\\s|$)|::)/i.test(line))continue;
    if(interpreter.test(line)) {
      problems.push("Python executable reference in active "+path+":"+number);
    }
  }
}
// Scanner regression: metadata must not fail CI; executable run blocks must.
const yamlFixture=[
  "jobs:",
  "  test:",
  "    steps:",
  "      - name: Enforce Rust only (no active Python)",
  "        run: |",
  "          echo ok",
  "          python blocked.py"
];
const executable=executableStatements(".github/workflows/rust.yml",yamlFixture);
assert.equal(executable.length,2);
assert.equal(executable.filter(({text})=>interpreter.test(text)).length,1);

const cap=JSON.parse(readFileSync("crates/pravda-app/capabilities/main.json","utf8"));
requireCheck(JSON.stringify(cap.windows)==='["main"]',
  "Only main may have Tauri IPC capability");
requireCheck(!("remote" in cap),"No remote capability allowed");
requireCheck(!cap.permissions.some(p=>/\b(?:fs|shell):/.test(p)),
  "No shell or filesystem plugin permission allowed");
if(problems.length>0) {
  for(const problem of problems)console.error("[RUST BOUNDARY FAIL] "+problem);
  process.exitCode=1;
} else {
  console.log("Rust production boundary OK");
  console.log("Archived Python sources: "+historical.filter(p=>p.endsWith(".py")).length);
  console.log("Active Python source files: 0");
  console.log("Active Python packaging files: 0");
  console.log("Checked Rust/Tauri entry points, CI and Windows launch scripts");
}
