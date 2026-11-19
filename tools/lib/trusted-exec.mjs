// Executable resolution for the repository's own test harnesses.
//
// Sonar javascript:S4036: a bare command name is resolved through the inherited
// PATH, which on Windows also searches the current directory. A `git.exe` or
// `tauri-driver.exe` dropped in a checkout (or any writable PATH entry) would
// then be executed by the security harnesses themselves.
//
// This module resolves a command ONLY inside a fixed list of absolute system
// directories, never through the inherited PATH and never from the working
// directory, and gives the child the same fixed PATH. Resolution failure is
// fatal: there is deliberately no PATH fallback.
import { execFileSync, spawn } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { isAbsolute, join, win32 } from "node:path";

/// Fixed, absolute, administrator-owned locations. No relative entry, no ".",
/// no caller-supplied directory, no environment-derived search path.
export function trustedDirectories(platform = process.platform) {
  return platform === "win32"
    ? ["C:\\Windows\\System32", "C:\\Windows\\System32\\WindowsPowerShell\\v1.0",
      "C:\\Program Files\\Git\\cmd", "C:\\Program Files\\Git\\bin",
      "C:\\Program Files (x86)\\Git\\cmd", "C:\\ProgramData\\chocolatey\\bin"]
    : ["/usr/bin", "/bin", "/usr/local/bin", "/usr/sbin", "/sbin"];
}

/// Narrow, EXPLICIT allowance for tools installed by the Rust toolchain
/// (`cargo install tauri-driver`, `cargo tauri`). A rustup toolchain lives in a
/// user-owned directory by design; it is named here rather than discovered
/// through PATH, so a checkout or a temporary directory can never supply it.
export function toolchainDirectories() {
  const home = process.env.CARGO_HOME || join(homedir(), ".cargo");
  return [join(home, "bin")];
}

// CI downloads the Microsoft-matched EdgeDriver to this exact directory.
// Resolve it from the runner's absolute temp root; never search inherited PATH.
export function edgeDriverDirectory(runnerTemp = process.env.RUNNER_TEMP, platform = process.platform) {
  if (platform !== "win32" || typeof runnerTemp !== "string" ||
      !win32.isAbsolute(runnerTemp)) {
    throw new Error("Windows E2E requires an absolute RUNNER_TEMP for matched EdgeDriver");
  }
  return win32.join(runnerTemp, "edgedriver-bin");
}

function isFile(path) {
  try { return existsSync(path) && statSync(path).isFile(); } catch { return false; }
}
function isDirectory(path) {
  try { return statSync(path).isDirectory(); } catch { return false; }
}

/// Absolute path of `command` inside the allowed directories, or null.
export function resolveTrusted(command, extraDirectories = [], platform = process.platform) {
  if (!/^[A-Za-z0-9._-]+$/.test(command)) {
    throw new Error("Command names must be plain identifiers: " + JSON.stringify(command));
  }
  const suffixes = platform === "win32" ? [".exe", ".cmd", ".bat", ""] : [""];
  for (const directory of [...trustedDirectories(platform), ...extraDirectories]) {
    for (const suffix of suffixes) {
      const candidate = join(directory, command + suffix);
      if (isFile(candidate)) return candidate;
    }
  }
  return null;
}

function fixed(command, options) {
  const extra = options.extraDirectories || [];
  // Child-only dependency search paths do not participate in the resolution of
  // the invoked command, so an EdgeDriver directory cannot supply tauri-driver.
  const dependent = options.dependentDirectories || [];
  if (!Array.isArray(dependent) || dependent.some(dir =>
      typeof dir !== "string" || !isAbsolute(dir) || !isDirectory(dir))) {
    throw new Error("Dependent executable directories must exist and be absolute");
  }
  const executable = resolveTrusted(command, extra);
  if (!executable) {
    throw new Error(`Trusted executable not found for ${command}; ` +
      "refusing to fall back to the inherited PATH");
  }
  const separator = process.platform === "win32" ? ";" : ":";
  const path = [...trustedDirectories(), ...extra, ...dependent].join(separator);
  const rest = { ...options };
  delete rest.extraDirectories;
  delete rest.dependentDirectories;
  return [executable, { ...rest, env: { ...rest.env, PATH: path, Path: path } }];
}

/// Drop-in replacement for execFileSync that cannot be hijacked by PATH.
export function trustedExecFileSync(command, args, options = {}) {
  const [executable, resolved] = fixed(command, options);
  return execFileSync(executable, args, resolved);
}

/// Drop-in replacement for spawn that cannot be hijacked by PATH.
export function trustedSpawn(command, args, options = {}) {
  const [executable, resolved] = fixed(command, options);
  return spawn(executable, args, resolved);
}
