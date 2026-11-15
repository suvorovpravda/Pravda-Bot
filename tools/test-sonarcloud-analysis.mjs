#!/usr/bin/env node
// Guard SonarCloud automatic-analysis scope: keep active Rust, frontend and
// tooling covered while excluding only the retired Python archive.
import assert from "node:assert/strict";
import {existsSync, readFileSync} from "node:fs";

const file=".sonarcloud.properties";
assert.ok(existsSync(file), "SonarCloud automatic analysis configuration must exist");
const config=new Map();
for (const line of readFileSync(file,"utf8").split(/\r?\n/)) {
  const trimmed=line.trim();
  if (!trimmed || trimmed.startsWith("#")) continue;
  const equal=trimmed.indexOf("=");
  assert.ok(equal>0,"Only key=value property entries allowed");
  const key=trimmed.slice(0,equal).trim();
  assert.ok(!config.has(key),"Duplicate SonarCloud property: "+key);
  config.set(key,trimmed.slice(equal+1).trim());
}
assert.equal(config.get("sonar.python.version"),"3.12",
  "The archived Python project requires Python >=3.12,<3.13");
assert.match(readFileSync("archive/python-legacy/root-packaging/pyproject.toml","utf8"),
  /requires-python\s*=\s*">=3\.12,<3\.13"/);
assert.equal(config.get("sonar.sources"),".",
  "Keep source audit at repository root, including active Rust and JavaScript");
const testRoots=["tests_js"];
assert.deepEqual(config.get("sonar.tests")?.split(",").map(x=>x.trim()).sort((a,b)=>a.localeCompare(b)),
  testRoots.slice().sort((a,b)=>a.localeCompare(b)),"Mark both Python and JS test roots as test code");
for (const dir of testRoots) assert.ok(existsSync(dir),"Test directory is absent: "+dir);
assert.ok(existsSync("archive/python-legacy"),"Retired Python archive must remain in the repository");
assert.deepEqual(config.get("sonar.exclusions")?.split(",").map(x=>x.trim()).sort((a,b)=>a.localeCompare(b)),
  ["archive/**",...testRoots.map(x=>x+"/**")].sort((a,b)=>a.localeCompare(b)),
  "Exclude only the retired archive and active test roots from production-source scope");
assert.equal(config.has("sonar.test.exclusions"),false,
  "Do not hide the test suite from SonarCloud's test-code analysis");
assert.equal(config.has("sonar.coverage.exclusions"),false,
  "Do not fabricate coverage by excluding application code");
console.log("SonarCloud analysis scope OK: active Rust/JS/tools audited, retired Python archive excluded, active JS tests separately indexed");
