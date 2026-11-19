// Regression: the security harnesses must never execute a command supplied by
// the working directory or by a writable PATH entry (Sonar javascript:S4036).
// Run: node --test tools/tests/trusted-exec.test.mjs
// The hijack attempt below is really executed, not simulated.
import test from "node:test";
import assert from "node:assert/strict";
import { chmodSync, existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  resolveTrusted, trustedDirectories, trustedExecFileSync, edgeDriverDirectory
} from "../lib/trusted-exec.mjs";

test("a command is resolved only inside the fixed system directories", () => {
  const resolved = resolveTrusted("git");
  assert.ok(resolved, "git must be resolvable on a development or CI machine");
  assert.ok(trustedDirectories().some(dir => resolved.startsWith(dir)),
    "the resolved executable must live in an allowed absolute directory");
  assert.ok(!resolved.startsWith(process.cwd()),
    "the working directory must never supply the executable");
});

test("a decoy executable earlier in PATH is never executed", () => {
  const directory = mkdtempSync(join(tmpdir(), "pravda-hijack-"));
  const marker = join(directory, "HIJACKED");
  const decoy = join(directory, process.platform === "win32" ? "git.cmd" : "git");
  writeFileSync(decoy, process.platform === "win32"
    ? `@echo off\r\necho hijacked > "${marker}"\r\n`
    : `#!/bin/sh\necho hijacked > "${marker}"\n`);
  chmodSync(decoy, 0o755);
  const savedPath = process.env.PATH;
  const savedCwd = process.cwd();
  try {
    process.env.PATH = directory + (process.platform === "win32" ? ";" : ":") + savedPath;
    process.chdir(directory);
    const resolved = resolveTrusted("git");
    assert.notEqual(resolved, decoy, "PATH must not influence resolution");
    const version = trustedExecFileSync("git", ["--version"], { encoding: "utf8" });
    assert.match(String(version), /git version/, "the real git must run");
    assert.ok(!existsSync(marker), "the decoy executable must never have run");
  } finally {
    process.chdir(savedCwd);
    process.env.PATH = savedPath;
  }
});

test("an unresolvable command fails closed instead of falling back to PATH", () => {
  const directory = mkdtempSync(join(tmpdir(), "pravda-fallback-"));
  const name = "pravda-nonexistent-tool";
  const decoy = join(directory, process.platform === "win32" ? name + ".cmd" : name);
  writeFileSync(decoy, process.platform === "win32" ? "@echo off\r\n" : "#!/bin/sh\n");
  chmodSync(decoy, 0o755);
  const savedPath = process.env.PATH;
  try {
    process.env.PATH = directory;
    assert.throws(() => trustedExecFileSync(name, []), /refusing to fall back/,
      "a tool present only in PATH must be refused");
  } finally { process.env.PATH = savedPath; }
});

test("command names carrying a path are rejected outright", () => {
  for (const name of ["../git", "sub/git", "..\\git", "git;rm", "git.sh "]) {
    assert.throws(() => resolveTrusted(name), /plain identifiers/,
      `${JSON.stringify(name)} must be refused`);
  }
});

test("child dependency PATH is explicitly scoped and never inherits attacker PATH", () => {
  const allowed = mkdtempSync(join(tmpdir(), "pravda-dependent-"));
  const attacker = mkdtempSync(join(tmpdir(), "pravda-path-attacker-"));
  const cmd = process.platform === "win32" ? "cmd" : "env";
  const args = process.platform === "win32" ? ["/d", "/c", "echo %PATH%"] : [];
  const output = String(trustedExecFileSync(cmd, args, {
    encoding: "utf8",
    dependentDirectories: [allowed],
    env: { ...process.env, PATH: attacker, Path: attacker }
  }));
  assert.ok(output.includes(allowed), "the explicitly supplied helper path must survive");
  assert.ok(!output.includes(attacker), "attacker PATH must never be inherited");
  assert.throws(() => trustedExecFileSync(cmd, args, {
    dependentDirectories: ["relative/path"]
  }), /absolute/);
});

test("the Windows EdgeDriver path derives only from an absolute runner temp root", () => {
  assert.equal(edgeDriverDirectory("D:\\a\\_temp", "win32"),
    "D:\\a\\_temp\\edgedriver-bin");
  assert.throws(() => edgeDriverDirectory(".\\_temp", "win32"),
    /absolute RUNNER_TEMP/);
});
