# Windows updater metadata — preparation gate

**Current state: not activated.** The installed desktop does not yet have a configured Tauri 2
updater plugin, public verification key, or authorized stable update channel.
Do not claim that automatic updates already work.

## Purpose

The CI-only `tools/prepare-updater-release.mjs` prepares a Tauri static `latest.json`
from an existing signed Windows MSI/NSIS bundle and its corresponding `.sig` file.
It also produces `latest.provenance.json` with the precise Git SHA and SHA-256 evidence.

The tool validates the stable release SemVer, matching tag and desktop version,
installer header and SHA-256, signature-file presence and a fixed GitHub HTTPS
release URL. **It DOES NOT cryptographically verify a signature or Authenticode.**
A `.sig` file beside a package is not proof that the package is authentic.

## Preparation command (example, not a production release)

    node tools/prepare-updater-release.mjs \
      --version 0.3.4 --tag v0.3.4 \
      --commit <exact-40-character-lowercase-git-sha> \
      --installer <verified-installer.msi> \
      --signature <verified-installer.msi.sig> \
      --sha256 <64-character-lowercase-sha256> \
      --output-dir <existing-empty-release-directory>

Inputs must originate from an authenticated build for the exact release commit;
independently verify the package hash against the final SHA256SUMS evidence.
The example does not claim any signed stable release is available.

Tauri 2 expects a `platforms.windows-x86_64` entry containing a trusted release
URL and the **text content** of the `.sig` file, not a path to that file.
The JSON sidecar explicitly records `cryptographic_signature_verified: false`
and `authenticode_verified: false` because metadata construction is not verification.

## Remaining mandatory gates before automatic installation

- Integrate the official Tauri 2 Rust updater plugin and lock its dependencies.
- Protect a real release private key in the signing environment. Embed only the
  corresponding public verification key in the application. Never commit test
  keys or passphrase/password material, and never echo credentials into logs.
- Create signed updater packages with the official signer; independently
  validate their authenticity before publishing release metadata.
- Validate Windows Authenticode with actual timestamp separately where required.
- Reject missing/bad signatures, unexpected origins, prerelease/downgrade
  versions and mismatched tags or artifact hashes; do not publish by default.
- Keep offline operation functional if the update check cannot reach HTTPS.
- Test installation, restart, SQLite/auth persistence and recovery on separate
  disposable Windows VM snapshots without representing hosted CI as physical QA.
- Update installation must never enable real orders or withdrawals.

References: https://v2.tauri.app/plugin/updater/
