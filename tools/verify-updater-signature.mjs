// CI-only verification of the Tauri v2 Minisign format. No private key, network
// access or installer execution is needed. The installed updater must still
// perform its own verification before installation.
import assert from "node:assert/strict";
import { createHash, createPublicKey, verify } from "node:crypto";
import { TextDecoder } from "node:util";

const ED25519_SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");
const decoder = new TextDecoder("utf-8", {fatal: true});

function decodeBase64(input, label) {
  assert.ok(typeof input === "string" && input.length > 0 && input.length <= 4096,
    label + " is missing or too large");
  assert.ok(/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(input),
    label + " must be canonical base64");
  const result = Buffer.from(input, "base64");
  assert.ok(result.toString("base64") === input, label + " must be canonical base64");
  return result;
}

function decodeLines(encoded, label, count) {
  let decoded;
  try { decoded = decoder.decode(decodeBase64(encoded, label)); }
  catch { throw new Error(label + " is not a valid encoded Minisign document"); }
  assert.ok(!/PRIVATE (?:KEY|CERTIFICATE)|BEGIN [A-Z ]*PRIVATE/i.test(decoded),
    label + " must not contain private key material");
  const lines = decoded.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop();
  assert.ok(lines.length === count && lines.every(line => !line.includes("\r")),
    label + " has an invalid document structure");
  return lines;
}

/** Verify Tauri's prehashed artifact signature and its signed version/filename.
 * digest is the BLAKE2b-512 hash of the exact installer bytes.
 */
export function verifyTauriUpdaterSignature({ publicKey, signature, digest, version, filename }) {
  assert.ok(Buffer.isBuffer(digest) && digest.length === 64, "Invalid installer digest");
  const keyLines = decodeLines(typeof publicKey === "string" ? publicKey.trim() : publicKey,
    "Updater public key", 2);
  const signatureLines = decodeLines(typeof signature === "string" ? signature.trim() : signature,
    "Updater signature", 4);
  assert.ok(keyLines[0].startsWith("untrusted comment: "), "Invalid public key comment");
  assert.ok(signatureLines[0].startsWith("untrusted comment: "), "Invalid signature comment");
  assert.ok(signatureLines[2].startsWith("trusted comment: "), "Missing signed comment");
  const keyPacket = decodeBase64(keyLines[1], "Updater public key packet");
  const sigPacket = decodeBase64(signatureLines[1], "Updater signature packet");
  const globalSignature = decodeBase64(signatureLines[3], "Updater comment signature");
  assert.ok(keyPacket.length === 42 && sigPacket.length === 74 && globalSignature.length === 64,
    "Invalid updater key or signature packet length");
  assert.ok(keyPacket.subarray(0, 2).equals(Buffer.from("Ed")) &&
    sigPacket.subarray(0, 2).equals(Buffer.from("ED")),
    "Only prehashed Ed25519 updater signatures are accepted");
  assert.ok(keyPacket.subarray(2, 10).equals(sigPacket.subarray(2, 10)),
    "Updater signing key does not match the configured public key");

  const key = createPublicKey({
    key: Buffer.concat([ED25519_SPKI_PREFIX, keyPacket.subarray(10)]),
    format: "der", type: "spki"
  });
  const artifactSignature = sigPacket.subarray(10);
  const trustedComment = signatureLines[2].slice("trusted comment: ".length);
  assert.ok(verify(null, digest, key, artifactSignature), "Updater artifact signature is invalid");
  assert.ok(verify(null, Buffer.concat([artifactSignature, Buffer.from(trustedComment, "utf8")]),
    key, globalSignature), "Updater trusted-comment signature is invalid");

  // Only inspect the comment after authenticating it. Reject duplicate fields
  // instead of letting different parsers disagree about the signed version.
  const fields = trustedComment.split("\t");
  const versions = fields.filter(field => field.startsWith("version:"));
  const files = fields.filter(field => field.startsWith("file:"));
  assert.ok(versions.length === 1 && versions[0] === "version:" + version,
    "Signed updater version does not match the release version");
  assert.ok(files.length === 1 && files[0] === "file:" + filename,
    "Signed updater filename does not match the installer");
  return {publicKeySha256:createHash("sha256").update(keyPacket.subarray(10)).digest("hex")};
}
