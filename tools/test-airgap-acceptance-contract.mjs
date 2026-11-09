// Source-level offline VM acceptance invariants, run on Linux and Windows CI.
// Does NOT claim to have run the actual hypervisor-isolated Windows VM.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const read = name => readFileSync(new URL(name, import.meta.url), "utf8");
const offline = read("./Test-Pravda-Airgap-Install.ps1");
const helper = read("./Initialize-Pravda-Airgap-Gui.ps1");
const msi = read("./Test-Pravda-Msi-Install.ps1");
const nsis = read("./Test-Pravda-Nsis-Install.ps1");
const workflow = read("../.github/workflows/windows-package.yml");
const gui = read("./test-installed-onboarding.mjs");
assert.match(workflow,/PRAVDA_SOURCE_SHA: \$\\{\\{ github\\.event\\.pull_request\\.head\\.sha \\|\\| github\\.sha \\}\\}/,
  "Windows CI must stamp the actual source HEAD, not synthetic PR merge SHA");
assert.match(gui,/sha: process\\.env\\.PRAVDA_SOURCE_SHA \\|\\| ""/,
  "Installed GUI report must use exact immutable source SHA");
for (const script of [msi,nsis]) {
  assert.match(script,/\\$uiEvidence\\.sha -cne \\$env:PRAVDA_SOURCE_SHA/,
    "Installed GUI acceptance must compare report against the source HEAD");
  assert.match(script,/Installed GUI acceptance requires the exact source SHA/,
    "Missing source SHA must fail closed");
}
assert.match(offline,/\\$env:PRAVDA_SOURCE_SHA = \\$ExpectedCommitSha\\.ToLowerInvariant\\(\\)/,
  "Airgap VM must bind its report to the externally trusted release SHA");

for (const required of ["ExpectedCommitSha","GITHUB_SHA",
  "PRAVDA_AIRGAP_ACCEPTANCE","Initialize-Pravda-Airgap-Gui.ps1",
  "msedgedriver.exe","tauri-driver.exe","node.exe",
  "Get-NetAdapter","Get-NetRoute","Get-FileHash"]) {
  assert.ok(offline.includes(required), "Missing offline prerequisite: "+required);
}
for (const required of ["WebView2 runtime missing after offline installer",
  "staged EdgeDriver major does not match installed WebView2",
  "PRAVDA_E2E_WEBVIEW2_VERSION","GITHUB_SHA",
  "Get-NetAdapter","Get-NetRoute"]) {
  assert.ok(helper.includes(required), "Missing post-install GUI preflight: "+required);
}
assert.doesNotMatch(helper,/Invoke-WebRequest|Invoke-RestMethod|Start-BitsTransfer/,
  "GUI helper must never attempt to fetch anything in offline VM");
for (const [name,source] of [["MSI",msi],["NSIS",nsis]]) {
  assert.ok(source.includes("PRAVDA_AIRGAP_ACCEPTANCE -ceq 'true'"),
    name+" must detect explicitly offline VM acceptance");
  assert.ok(source.includes("Initialize-Pravda-Airgap-Gui.ps1"),
    name+" must verify offline GUI prerequisites after WebView2 installation");
  assert.ok(source.includes("node (Join-Path $PSScriptRoot 'test-installed-onboarding.mjs')"),
    name+" must use its own local test script rather than current directory");
}
for (const required of ["pravda-airgap-evidence-", "installed-gui-report.json",
  "installed_gui_report_sha256", "installer_sha256", "source_sha",
  "GUI acceptance proof is inconsistent"]) {
  assert.ok(offline.includes(required),
    "Offline run must preserve separately hashed GUI evidence: "+required);
}
// A functional offline VM PASS must never silently become a signed-release PASS.
for (const required of ["RequireSigned", "ExpectedSignerThumbprint",
  "Get-AuthenticodeSignature", "SignerCertificate.Thumbprint",
  "TimeStamperCertificate", "installer_authenticode_verified",
  "release_ready = $false", "unsigned_or_unverified_functional_qa",
  "FUNCTIONAL QA ONLY", "REFUSED: installer Authenticode"]) {
  assert.ok(offline.includes(required),
    "Offline QA must distinguish signed and unsigned release evidence: "+required);
}
const signatureCheck = offline.indexOf("if ($RequireSigned) {");
const beforeInstall = offline.indexOf("# Test-only toolchain must be staged");
assert.ok(signatureCheck >= 0 && signatureCheck < beforeInstall,
  "Signature must be checked before executing the offline installer");
assert.ok(workflow.includes("Initialize-Pravda-Airgap-Gui.ps1"),
  "Windows CI must parse the offline GUI helper");
assert.ok(workflow.includes("-ExpectedCommitSha ('0' * 40)"),
  "Negative acknowledgement gate must pass the required SHA parameter");
console.log("Offline VM installed-GUI source contracts OK (NOT a physical VM acceptance)");
