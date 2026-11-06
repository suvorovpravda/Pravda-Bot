// CI-only preparation of Tauri's static Windows updater metadata.
// A .sig file is required but only the installed Tauri updater, configured
// with the trusted public key, can authenticate the update cryptographically.
// This tool NEVER accepts a private key or installs a package.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  createReadStream, existsSync, lstatSync, openSync, closeSync, readSync,
  readFileSync, writeFileSync, unlinkSync
} from "node:fs";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const PUBLIC_ORIGIN = "https://github.com/suvorovpravda/Pravda-Bot/releases/download";
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const SHA256 = /^[0-9a-f]{64}$/;
const COMMIT_SHA = /^[0-9a-f]{40}$/;
const FILE_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,150}$/;

function safeFile(path, label) {
  assert.equal(typeof path, "string", label + " must be a file path");
  assert.ok(path.length > 0, label + " path is empty");
  const stat = lstatSync(path);
  assert.ok(stat.isFile() && !stat.isSymbolicLink(), label + " must be a regular file, not a symlink");
  return stat;
}

function installerHeader(path, suffix) {
  const bytes = Buffer.alloc(8);
  const fd = openSync(path, "r");
  try {
    assert.equal(readSync(fd, bytes, 0, 8, 0), 8, "Installer header is truncated");
  } finally {
    closeSync(fd);
  }
  if (suffix === ".msi") {
    assert.ok(bytes.equals(Buffer.from("d0cf11e0a1b11ae1", "hex")), "MSI OLE magic mismatch");
  } else {
    assert.ok(bytes.subarray(0, 2).equals(Buffer.from("MZ")), "Installer PE magic mismatch");
  }
}

async function sha256OfFile(path) {
  const hasher = createHash("sha256");
  for await (const chunk of createReadStream(path)) hasher.update(chunk);
  return hasher.digest("hex");
}

export async function buildUpdaterMetadata(options) {
  const { version, commit, tag, installerPath, signaturePath, expectedSha256, expectedAppVersion } = options;
  assert.ok(VERSION.test(version), "Only stable SemVer releases are allowed");
  assert.equal(version, expectedAppVersion, "Release version must match the desktop bundle version");
  assert.equal(tag, "v" + version, "Git tag must match the bundled stable version");
  assert.ok(COMMIT_SHA.test(commit), "Full lowercase Git commit SHA is required");
  assert.ok(SHA256.test(expectedSha256), "Full lowercase SHA-256 is required");

  const filename = basename(installerPath);
  assert.ok(FILE_NAME.test(filename), "Unsafe release installer filename");
  const isMsi = filename.endsWith(".msi");
  const isNsis = filename.endsWith("-setup.exe");
  assert.ok(isMsi || isNsis, "Only actual Windows MSI or NSIS installer assets are allowed");
  assert.equal(resolve(signaturePath), resolve(installerPath + ".sig"),
    "Signature must correspond to this exact installer file");
  const stat = safeFile(installerPath, "Installer");
  assert.ok(stat.size >= 100000 && stat.size <= 2 * 1024 * 1024 * 1024,
    "Installer has an implausible size");
  installerHeader(installerPath, isMsi ? ".msi" : ".exe");
  const digest = await sha256OfFile(installerPath);
  assert.equal(digest, expectedSha256, "Installer SHA-256 does not match the release evidence");

  const sigStat = safeFile(signaturePath, "Updater signature");
  assert.ok(sigStat.size >= 64 && sigStat.size <= 4096,
    "Updater signature is missing or has an implausible size");
  const signature = readFileSync(signaturePath, "utf8").trim();
  assert.ok(signature.length >= 64 && /^[\x20-\x7E\r\n]+$/.test(signature),
    "Updater signature contains invalid bytes");
  assert.doesNotMatch(signature, /PRIVATE (?:KEY|CERTIFICATE)|BEGIN [A-Z ]*PRIVATE/i,
    "A private key must never be published as an updater signature");
  assert.ok(signature.split(/\r?\n/).some(line => /^[A-Za-z0-9+/=]{64,}$/.test(line)),
    "Updater signature has no plausible encoded signature line");
  const signatureHash = createHash("sha256").update(readFileSync(signaturePath)).digest("hex");

  // The JSON signature field contains the SIGNATURE CONTENT, never a path/URL.
  const url = PUBLIC_ORIGIN + "/" + encodeURIComponent(tag) + "/" + encodeURIComponent(filename);
  const manifest = {
    version,
    notes: "Pravda-Bot Windows stable update. Install only with Tauri signature verification.",
    platforms: { "windows-x86_64": { url, signature } }
  };
  const provenance = {
    schema_version: 1, version, tag, commit_sha: commit,
    platform: "windows-x86_64", asset: filename,
    artifact_sha256: digest, signature_sha256: signatureHash,
    cryptographic_signature_verified: false,
    authenticode_verified: false,
    note: "Manifest preparation only. No cryptographic or physical Windows acceptance is asserted."
  };
  return { manifest, provenance };
}

export function writeUpdaterMetadata(outputDir, metadata) {
  const latest = join(outputDir, "latest.json");
  const evidence = join(outputDir, "latest.provenance.json");
  assert.ok(!existsSync(latest) && !existsSync(evidence),
    "Refusing to overwrite existing updater release metadata");
  let evidenceCreated = false;
  try {
    writeFileSync(evidence, JSON.stringify(metadata.provenance, null, 2) + "\n", {flag: "wx", mode: 0o600});
    evidenceCreated = true;
    writeFileSync(latest, JSON.stringify(metadata.manifest, null, 2) + "\n", {flag: "wx", mode: 0o600});
  } catch (error) {
    if (evidenceCreated) unlinkSync(evidence);
    throw error;
  }
  return { latest, evidence };
}

async function main(args) {
  const known = new Set(["version", "commit", "tag", "installer", "signature", "sha256", "output-dir"]);
  const input = {};
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i]?.replace(/^--/, "");
    assert.ok(args[i]?.startsWith("--") && known.has(key) && args[i + 1] &&
      !args[i + 1].startsWith("--") && !(key in input), "Invalid or repeated release argument");
    input[key] = args[i + 1];
  }
  assert.equal(Object.keys(input).length, known.size, "Every release argument is required");
  const config = JSON.parse(readFileSync("crates/pravda-app/tauri.conf.json", "utf8"));
  const metadata = await buildUpdaterMetadata({
    version: input.version, commit: input.commit, tag: input.tag,
    installerPath: input.installer, signaturePath: input.signature,
    expectedSha256: input.sha256, expectedAppVersion: config.version
  });
  const written = writeUpdaterMetadata(input["output-dir"], metadata);
  process.stdout.write("Prepared Windows updater metadata: " + written.latest + "\n");
  process.stdout.write("Updater cryptographic verification and Authenticode: NOT VERIFIED by this tool\n");
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  main(process.argv.slice(2)).catch(error => {
    process.stderr.write("Updater metadata refused: " + error.message + "\n");
    process.exitCode = 1;
  });
}
