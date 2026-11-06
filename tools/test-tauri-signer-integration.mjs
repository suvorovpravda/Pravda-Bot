// CI-only compatibility check using the installed, pinned Tauri CLI signer.
// The temporary signing key is generated on the runner, never printed or
// included in artifacts, and removed with its disposable test directory.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { buildUpdaterMetadata } from "./prepare-updater-release.mjs";

const root = mkdtempSync(join(tmpdir(), "pravda-updater-signer-test-"));
const keyPath = join(root, "throwaway.key");
const installer = join(root, "Pravda-Bot_0.3.4_x64_en-US.msi");
const version = "0.3.4";
const env = {...process.env};
for (const name of ["TAURI_SIGNING_PRIVATE_KEY", "TAURI_SIGNING_PRIVATE_KEY_PATH",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "TAURI_PRIVATE_KEY", "TAURI_PRIVATE_KEY_PATH",
  "TAURI_PRIVATE_KEY_PASSWORD"]) delete env[name];

function signer(args) {
  try {
    execFileSync("cargo", ["tauri", "signer", ...args], {
      env, stdio: ["ignore", "pipe", "pipe"], timeout: 60000, maxBuffer: 1024 * 1024
    });
  } catch {
    // Never relay a subprocess environment or the test key in logs.
    throw new Error("Tauri signer integration test failed");
  }
}

try {
  signer(["generate", "--ci", "--password", "", "--write-keys", keyPath]);
  const payload = Buffer.alloc(130000);
  Buffer.from("d0cf11e0a1b11ae1", "hex").copy(payload);
  writeFileSync(installer, payload);
  signer(["sign", "--private-key-path", keyPath, "--password", "", "--app-version", version, installer]);
  const publicKey = readFileSync(keyPath + ".pub", "utf8").trim();
  const hash = () => createHash("sha256").update(readFileSync(installer)).digest("hex");
  const options = {version, tag:"v" + version, commit:"a".repeat(40),
    expectedAppVersion:version, installerPath:installer, signaturePath:installer + ".sig", publicKey};
  const result = await buildUpdaterMetadata({...options, expectedSha256:hash()});
  assert.equal(result.provenance.cryptographic_signature_verified, true);
  assert.equal(result.provenance.authenticode_verified, false);
  payload[200] = 1;
  writeFileSync(installer, payload);
  await assert.rejects(buildUpdaterMetadata({...options, expectedSha256:hash()}), /artifact signature is invalid/);
  console.log("PASS official Tauri signer: generated signature accepted; altered installer rejected");
} finally {
  rmSync(root, {recursive:true, force:true});
}
