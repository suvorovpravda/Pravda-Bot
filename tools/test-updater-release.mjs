import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { tmpdir } from "node:os";
import { buildUpdaterMetadata, writeUpdaterMetadata } from "./prepare-updater-release.mjs";
import { verifyTauriUpdaterSignature } from "./verify-updater-signature.mjs";

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
assert.match(workflowSource, /TAURI_UPDATER_PUBLIC_KEY: \$\{\{ secrets\.TAURI_SIGNING_PUBLIC_KEY \}\}/,
  "The signed-tag Tauri config must receive the protected public signing key");
assert.match(workflowSource, /pubkey = \$env:TAURI_UPDATER_PUBLIC_KEY\.Trim\(\)/,
  "Tauri CLI must get the verification pubkey when producing update signatures");
assert.match(workflowSource, /\$signingConfig\.plugins\.updater\.pubkey -cne \$env:TAURI_UPDATER_PUBLIC_KEY\.Trim\(\)/,
  "The metadata verification key must equal the public key supplied to the build");
assert.match(workflowSource, /endpoints = @\('https:\/\/github\.com\/suvorovpravda\/Pravda-Bot\/releases\/latest\/download\/latest\.json'\)/,
  "The updater channel must remain the fixed GitHub latest.json asset");
assert.match(workflowSource,/requireSignedVersion = \$true/,
  "Tauri signer metadata must enforce signed release version binding");
assert.match(workflowSource,/allowDowngrades = \$false/,
  "No update path may permit version rollback");
assert.match(workflowSource,/dangerousInsecureTransportProtocol = \$false/,
  "Tauri updater must never permit non-HTTPS endpoints");



const msi = join(root, "Pravda-Bot_0.3.4_x64_en-US.msi");
const sig = msi + ".sig";
// Keys are generated for this test run only and are never written to disk.
const {publicKey: testKey, privateKey: testSigner} = generateKeyPairSync("ed25519");
const testKeyId = Buffer.from("0123456789abcdef", "hex");
const keyPacket = Buffer.concat([Buffer.from("Ed"), testKeyId,
  testKey.export({type:"spki",format:"der"}).subarray(-32)]);
const publicKey = Buffer.from("untrusted comment: ephemeral test key\n" +
  keyPacket.toString("base64") + "\n").toString("base64");
function makeSignature(data, filename, commentVersion = version, extra = "") {
  const digest = createHash("blake2b512").update(data).digest();
  const signed = sign(null, digest, testSigner);
  const comment = "timestamp:1\tfile:" + filename +
    (commentVersion === null ? "" : "\tversion:" + commentVersion) + extra;
  const global = sign(null, Buffer.concat([signed, Buffer.from(comment)]), testSigner);
  return Buffer.from("untrusted comment: signature from tauri secret key\n" +
    Buffer.concat([Buffer.from("ED"), testKeyId, signed]).toString("base64") + "\n" +
    "trusted comment: " + comment + "\n" + global.toString("base64") + "\n").toString("base64");
}
const bytes = Buffer.alloc(130000);
Buffer.from("d0cf11e0a1b11ae1", "hex").copy(bytes);
const validSignature = makeSignature(bytes, basename(msi));
const hash = () => createHash("sha256").update(readFileSync(msi)).digest("hex");
const options = () => ({
  version, tag, commit, installerPath: msi, signaturePath: sig,
  expectedSha256: hash(), expectedAppVersion: version, publicKey
});
async function fails(patch, reason) {
  await assert.rejects(buildUpdaterMetadata({...options(), ...patch}), reason);
}

try {
  // Independent known-answer vector, produced by the upstream Tauri signer.
  const knownKey = Buffer.from("untrusted comment: minisign public key: A49EF4746FA6E6D7\n" +
    "RWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe\n").toString("base64");
  const knownSignature = Buffer.from("untrusted comment: signature from tauri secret key\n" +
    "RUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\n" +
    "trusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\n" +
    "DDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==\n").toString("base64");
  const known = {publicKey: knownKey, signature: knownSignature,
    digest: createHash("blake2b512").update("test").digest(), version:"2.0.0", filename:"payload"};
  verifyTauriUpdaterSignature(known);
  assert.throws(() => verifyTauriUpdaterSignature({...known, version:"9.0.0"}), /version/);
  assert.throws(() => verifyTauriUpdaterSignature({...known, digest:Buffer.alloc(64)}), /artifact signature/);

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
  assert.equal(valid.provenance.public_key_sha256,
    createHash("sha256").update(keyPacket.subarray(10)).digest("hex"));
  assert.equal(valid.provenance.cryptographic_signature_verified, true,
    "Metadata generation must record successful Tauri signature verification");
  assert.equal(valid.provenance.authenticode_verified, false,
    "Tauri update signatures cannot be treated as Authenticode verification");
  assert.ok(!JSON.stringify(valid).includes("BEGIN PRIVATE KEY"));

  const destination = join(root, "output");
  mkdirSync(destination);
  const paths = writeUpdaterMetadata(destination, valid);
  assert.deepEqual(JSON.parse(readFileSync(paths.latest, "utf8")), valid.manifest);
  assert.deepEqual(JSON.parse(readFileSync(paths.evidence, "utf8")), valid.provenance);
  assert.throws(() => writeUpdaterMetadata(destination, valid), /Refusing to overwrite/);

  // Exercise the actual CLI/environment boundary, not just its helper.
  const cliOutput = join(root, "cli-output");
  mkdirSync(cliOutput);
  const args = ["tools/prepare-updater-release.mjs", "--version", version, "--tag", tag,
    "--commit", commit, "--installer", msi, "--signature", sig, "--sha256", hash(), "--output-dir", cliOutput];
  const noKeyEnv = {...process.env};
  delete noKeyEnv.TAURI_UPDATER_PUBLIC_KEY;
  const missingKey = spawnSync(process.execPath, args, {env:noKeyEnv, encoding:"utf8"});
  assert.notEqual(missingKey.status, 0, "CLI must refuse release metadata without its public key");
  assert.equal(existsSync(join(cliOutput, "latest.json")), false, "A failed check must not publish metadata");
  const withKey = spawnSync(process.execPath, args, {
    env:{...noKeyEnv, TAURI_UPDATER_PUBLIC_KEY:publicKey}, encoding:"utf8"
  });
  assert.equal(withKey.status, 0, "CLI must accept a correctly signed installer");
  assert.ok(!withKey.stdout.includes(publicKey) && !withKey.stderr.includes(publicKey),
    "CLI must not print the configured key");
  assert.equal(JSON.parse(readFileSync(join(cliOutput,"latest.provenance.json"),"utf8"))
    .cryptographic_signature_verified, true);

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
  await fails({}, /artifact signature is invalid/);
  writeFileSync(msi, bytes);

  await fails({publicKey:undefined}, /Updater public key/);
  const unrelated = generateKeyPairSync("ed25519").publicKey.export({type:"spki",format:"der"}).subarray(-32);
  const wrongKeyPacket = Buffer.concat([Buffer.from("Ed"), testKeyId, unrelated]);
  const wrongKey = Buffer.from("untrusted comment: other public key\n" +
    wrongKeyPacket.toString("base64") + "\n").toString("base64");
  await fails({publicKey:wrongKey}, /artifact signature is invalid/);
  const wrongIdPacket = Buffer.from(keyPacket);
  wrongIdPacket[2] ^= 1;
  const wrongIdKey = Buffer.from("untrusted comment: other key id\n" +
    wrongIdPacket.toString("base64") + "\n").toString("base64");
  await fails({publicKey:wrongIdKey}, /signing key does not match/);
  writeFileSync(sig, makeSignature(bytes, basename(msi), "0.3.3"));
  await fails({}, /Signed updater version/);
  writeFileSync(sig, makeSignature(bytes, basename(msi), null));
  await fails({}, /Signed updater version/);
  writeFileSync(sig, makeSignature(bytes, "different.msi"));
  await fails({}, /Signed updater filename/);
  writeFileSync(sig, makeSignature(bytes, basename(msi), version, "\tversion:0.3.3"));
  await fails({}, /Signed updater version/);
  const tamperedComment = Buffer.from(validSignature, "base64").toString("utf8").replace("version:0.3.4", "version:9.0.0");
  writeFileSync(sig, Buffer.from(tamperedComment).toString("base64"));
  await fails({}, /trusted-comment signature is invalid/);

  writeFileSync(sig, "BEGIN PRIVATE KEY\n" + Buffer.alloc(96).toString("base64"));
  await fails({}, /private key|Updater signature/i);
  writeFileSync(sig, "bad signature");
  await fails({}, /implausible size/);
  writeFileSync(sig, validSignature);

  const nsis = join(root, "Pravda-Bot_0.3.4_x64-setup.exe");
  const executable = Buffer.alloc(140000);
  executable.write("MZ");
  writeFileSync(nsis, executable);
  writeFileSync(nsis + ".sig", makeSignature(executable, basename(nsis)));
  const nsisResult = await buildUpdaterMetadata({
    ...options(), installerPath: nsis, signaturePath: nsis + ".sig",
    expectedSha256: createHash("sha256").update(executable).digest("hex")
  });
  assert.ok(nsisResult.manifest.platforms["windows-x86_64"].url.endsWith("-setup.exe"));
  assert.ok(nsisResult.provenance.asset.endsWith("-setup.exe"));

  // The fixed release domain is never supplied by CLI or untrusted metadata.
  assert.doesNotMatch(JSON.stringify(nsisResult.manifest), /localhost|127\.0\.0\.1|http:/);
  console.log("PASS updater metadata: Tauri signature, signed version, key mismatch, tampering and separate Authenticode state");
} finally {
  rmSync(root, { recursive: true, force: true });
}
