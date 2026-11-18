import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

/**
 * Check JavaScript syntax without evaluating the source in this process.
 * `filename` is only a diagnostic label; it is never used as a disk path.
 */
export function validateSyntax(source, filename = "snippet.js") {
  assert.ok(typeof source === "string" && source.length > 0,
    "Source must be a non-empty string");
  assert.ok(typeof filename === "string" && filename.length > 0,
    "Filename must be a non-empty diagnostic label");

  const diagnosticName = filename.replace(/[\r\n]/g, "_");
  const tempDir = mkdtempSync(join(tmpdir(), "pravda-syntax-check-"));
  const tempFile = join(tempDir, "snippet.js");

  try {
    writeFileSync(tempFile, source, {
      encoding: "utf8",
      flag: "wx",
      mode: 0o600
    });

    try {
      execFileSync(process.execPath, ["--check", tempFile], {
        encoding: "utf8",
        stdio: ["ignore", "ignore", "pipe"],
        windowsHide: true
      });
    } catch (error) {
      // A syntax error is a non-zero Node exit. Preserve spawn, signal, and
      // other process failures as their original errors.
      if (!error || !Number.isInteger(error.status) || error.status === 0) {
        throw error;
      }

      const capturedStderr = typeof error.stderr === "string" ? error.stderr : "";
      const stderr = capturedStderr.split(tempFile).join(diagnosticName).trim();
      const syntaxError = new SyntaxError(
        stderr || `JavaScript syntax check failed for ${diagnosticName}`
      );
      syntaxError.code = "ERR_INVALID_JAVASCRIPT_SYNTAX";
      syntaxError.filename = diagnosticName;
      syntaxError.stderr = stderr;
      syntaxError.cause = error;
      throw syntaxError;
    }

    return true;
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
}
