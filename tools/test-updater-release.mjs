import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { buildUpdaterMetadata, writeUpdaterMetadata } from "./prepare-updater-release.mjs";

const root = mkdtempSync(join(tmpdir(), "pravda-updater-contract-"));
const version = "0.3.4", tag = "v0.3.4", commit = "c".repeat(40);
const workflowSource = readFileSync(".github/workflows/windows-package.yml", "utf8");
assert.match(workflowSource, /Require Tauri signer for tagged Windows updater candidates/,
  "Signed tagged updater builds require real protected signing prerequisites");
assert.match(workflowSource, /\$config\.bundle\.createUpdaterArtifacts = \$true/,
  "Updater artifacts are created only in explicitly signed tagged candidate builds");
assert.match(workflowSource, /TAURI_SIGNING_PRIVATE_KEY: \$\{\{ secrets\.TAURI_SIGNING_PRIVATE_KEY \}\}/,
  "Never embed a signing private key in source or plain workflow defaults");
assert.match(workflowSource, /name: pravda-signed-updater-metadata-candidate/,
  "Prepared metadata must be archived for review without automatic release publication");

const msi = join(root, "Pravda-Bot_0.3.4_x64_en-US.msi");
const sig = msi + ".sig";
const validSignature = "untrusted comment: signature from tauri signer\n" +
  Buffer.alloc(96, 0x5b).toString("base64") + "\n";
const bytes = Buffer.alloc(130000);
Buffer.from("d0cf11e0a1b11ae1", "hex").copy(bytes);
const hash = () => createHash("sha256").update(readFileSync(msi)).digest("hex");
const options = () => ({
  version, tag, commit, installerPath: msi, signaturePath: sig,
  expectedSha256: hash(), expectedAppVersion: version
});
async function fails(patch, reason) {
  await assert.rejects(buildUpdaterMetadata({...options(), ...patch}), reason);
}

try {
  writeFileSync(msi, bytes);
  writeFileSync(sig, validSignature);

  const valid = await buildUpdaterMetadata(options());
  assert.deepEqual(Object.keys(valid.manifest.platforms), ["windows-x86_64"]);
  assert.equal(valid.manifest.version, version);
  assert.equal(valid.manifest.platforms["windows-x86_64"].signature, validSignature.trim());
  assert.equal(valid.manifest.platforms["windows-x86_64"].url,
    "https://github.com/suvorovpravda/Pravda-Bot/releases/download/v0.3.4/" +
    "Pravda-Bot_0.3.4_x64_en-US.msi");
  assert.equal(valid.provenance.commit_sha, commit);
  assert.equal(valid.provenance.artifact_sha256, hash());
  assert.equal(valid.provenance.cryptographic_signature_verified, false,
    "Metadata generation must never claim cryptographic verification");
  assert.equal(valid.provenance.authenticode_verified, false,
    "Tauri update signatures cannot be treated as Authenticode verification");
  assert.ok(!JSON.stringify(valid).includes("BEGIN PRIVATE KEY"));

  const destination = join(root, "output");
  const {mkdirSync} = await import("node:fs");
  mkdirSync(destination);
  const paths = writeUpdaterMetadata(destination, valid);
  assert.deepEqual(JSON.parse(readFileSync(paths.latest, "utf8")), valid.manifest);
  assert.deepEqual(JSON.parse(readFileSync(paths.evidence, "utf8")), valid.provenance);
  assert.throws(() => writeUpdaterMetadata(destination, valid), /Refusing to overwrite/);

  await fails({version: "0.3.4-rc.1"}, /stable SemVer/);
  await fails({version: "01.3.4"}, /stable SemVer/);
  await fails({tag: "v0.3.5"}, /tag must match/);
  await fails({expectedAppVersion: "0.3.5"}, /bundle version/);
  await fails({commit: "x".repeat(40)}, /Git commit SHA/);
  await fails({expectedSha256: "0".repeat(64)}, /SHA-256 does not match/);
  await fails({signaturePath: join(root, "substitution.sig")}, /correspond to this exact/);

  // A changed installer must fail against previously recorded release evidence.
  const correctHash = hash();
  const corrupted = Buffer.from(bytes);
  corrupted[50] = 0xf1;
  writeFileSync(msi, corrupted);
  await fails({expectedSha256: correctHash}, /SHA-256 does not match/);
  writeFileSync(msi, bytes);

  writeFileSync(sig, "BEGIN PRIVATE KEY\n" + Buffer.alloc(96).toString("base64"));
  await fails({}, /private key|Updater signature/i);
  writeFileSync(sig, "bad signature");
  await fails({}, /implausible size/);
  writeFileSync(sig, validSignature);

  const nsis = join(root, "Pravda-Bot_0.3.4_x64-setup.exe");
  const executable = Buffer.alloc(140000);
  executable.write("MZ");
  writeFileSync(nsis, executable);
  writeFileSync(nsis + ".sig", validSignature);
  const nsisResult = await buildUpdaterMetadata({
    ...options(), installerPath: nsis, signaturePath: nsis + ".sig",
    expectedSha256: createHash("sha256").update(executable).digest("hex")
  });
  assert.ok(nsisResult.manifest.platforms["windows-x86_64"].url.endsWith("-setup.exe"));
  assert.ok(nsisResult.provenance.asset.endsWith("-setup.exe"));

  // The fixed release domain is never supplied by CLI or untrusted metadata.
  assert.doesNotMatch(JSON.stringify(nsisResult.manifest), /localhost|127\.0\.0\.1|http:/);
  console.log("PASS signed-updater metadata contracts: fixed origin, version, SHA, tampering and no false signature claims");
} finally {
  rmSync(root, { recursive: true, force: true });
}
