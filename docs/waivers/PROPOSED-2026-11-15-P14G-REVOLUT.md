# Proposed risk exception — P14G / optional Revolut (NOT APPROVED)

- **Created:** 2026-11-15
- **Status:** DRAFT FOR REVIEW — not a release authorization, not a change to #160
- **Owner / approver:** Unassigned; explicit human sign-off required
- **Tracking:** [#160](https://github.com/suvorovpravda/Pravda-Bot/issues/160), [#214](https://github.com/suvorovpravda/Pravda-Bot/pull/214), [#212](https://github.com/suvorovpravda/Pravda-Bot/pull/212), [#215](https://github.com/suvorovpravda/Pravda-Bot/pull/215), [#216](https://github.com/suvorovpravda/Pravda-Bot/pull/216)
- **Proposed target:** Windows test candidate only. No v0.3.5 production approval or date commitment.

## Scope and evidence required

1. **P14G / dashboard startup:** A previously installed MSI demonstrated `DASHBOARD_RANGE_L1 / STACK_OVERFLOW`. PR #214 is a candidate fix; a successful unit test or a disabled shadow logger does **not** demonstrate that the installed desktop starts. Keep bounded, redacted startup phase diagnostics enabled until the installed acceptance succeeds on the exact candidate SHA.
2. **Optional Revolut X public market book:** This feature must be opt-in, read-only, and disabled if its compile-time gate is absent, even when an earlier version persisted consent. The feature `revolut-consent` is enabled in normal builds for backward compatibility with existing acceptance. Build an isolated candidate with `--no-default-features` to evaluate the *disabled* mode; **no release workflow is changed by this proposal**.
3. **Extended security and Sonar tests:** Always ON. No `ENABLE_EXTENDED_SECURITY_TESTS=false`, no blanket `NOSONAR`, false-positive, or `Won't Fix` classifications. Review findings individually; do not lower the Quality Gate.
4. **External AI, trading and update rights:** No new AI network connector, broker execution, LIVE orders, credential exposure, updater activation, or network service is authorized.

## Acceptance matrix (all mandatory for any subsequent GO review)

- [ ] Pin exact commit SHA and immutable installed MSI/NSIS SHA-256; record artifact provenance.
- [ ] Run Rust unit tests for default and `--no-default-features` app configurations; verify the disabled Revolut handler refuses reads and preference writes while energy/news/sources keep working.
- [ ] Test installed Windows WebView2 login, first-use readiness, logout, restart, persistence, and uninstall/reinstall in both modes.
- [ ] Confirm no Revolut HTTP request is dispatched by Rust when compiled out; retain authenticated IPC checks and existing consent revocation behavior when compiled in.
- [ ] Complete CodeQL, RustSec, SonarCloud Quality Gate and native installer acceptance on **the same SHA**.
- [ ] Attach audit evidence for Authenticode signature and timestamp, or explicitly label unsigned test artifacts as **not distributable**.
- [ ] Obtain the required independent review and explicit owner/security approval in #160; document residual risks and rollback procedure.

## Regulatory applicability and non-negotiable internal controls (reviewed 2026-11-15)

This proposal **does not assert that Pravda-Bot is an authorized bank, a payment-service provider or a regulated financial entity**. Those classifications require an independent legal/scope assessment before offering any corresponding service.

| Framework | Applicability | Mandatory project control / evidence |
| --- | --- | --- |
| **PSD2 / Directive (EU) 2015/2366, Art. 67** | The article governs *payment-account information services* and requires the payment service user's explicit consent for that service. A read-only **public Revolut X order book** does not by itself establish that this article applies. Reassess if private payment-account access, payment initiation or financial credentials are introduced. | Regardless of PSD2 classification, the optional public-feed consent must be affirmative, revocable, checked in authenticated **Rust IPC**, and refused when the compile-time module is disabled. |
| **GDPR / Regulation (EU) 2016/679, Arts. 5 and 32** | Applicable to actual processing of **personal data** by a controller/processor. Public market prices alone are not necessarily personal data. Identify personal data processed (e.g. accounts, logs, consent identifiers) and the applicable role/legal basis before declaring compliance. | Minimize stored data, protect any personal credentials using OS-backed secret storage as appropriate, restrict diagnostic output, and test that consent revocation and session invalidation cannot leak personal or privileged data. |
| **Windows Authenticode / SmartScreen** | Microsoft code signing is a distribution-trust mechanism; **not** an unconditional general statute requiring every Windows executable to be signed. However, signed, timestamp-validated MSI/NSIS is a **binding Pravda-Bot #160 production-release policy**. | Record verified signer identity, chain/timestamp, matching SHA-256 of each installed artifact, installation/uninstallation checks and test provenance. Unsigned artifacts remain testing-only and non-distributable. |
| **OWASP / CWE and software supply chain** | Security guidance and weakness catalogues are **engineering references**, not independent statutory licensing regimes. Regulatory product requirements may apply separately depending on the business model and distribution. | Keep SonarCloud, CodeQL and RustSec gates, validate every IPC input in Rust, retain the security test harnesses, review each false-positive claim with evidence instead of blanket `NOSONAR` or `Won't Fix`. |
| **DORA / Regulation (EU) 2022/2554** | Scope depends on being a covered financial entity or a qualifying ICT third-party service provider and its contractual role; no such classification is established by the public-feed feature alone. | Document and separately review applicability before claiming DORA conformance. Keep critical dependencies, incident response and third-party ICT risk controls visible. |

**Official references**
- PSD2 consolidated Directive (EU) 2015/2366: https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A02015L2366-20250117
- GDPR Regulation (EU) 2016/679: https://eur-lex.europa.eu/eli/reg/2016/679
- Microsoft Windows app code-signing options: https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options
- ESMA DORA overview: https://www.esma.europa.eu/esmas-activities/digital-finance-and-innovation/digital-operational-resilience-act-dora

### Waiver cannot supersede release/security gates

- The proposed feature-flagged build is an **evaluation candidate**. Its safety has not been proven by creating a feature flag or writing this waiver.
- No automatic reinstatement of disabled modules, relaxing of the branch rules, signing requirement, Quality Gate, installation acceptance, or #160 NO-GO is permitted.
- Evidence must identify **the exact SHA, binary SHA-256, CI run/job IDs, signed installer and independent installed-client checks**.
- Only the designated project owner and security reviewer may record an explicit GO decision after the outstanding evidence is independently evaluated. Blank approvals or a review-date expiry do not count as consent, compliance certification or production clearance.

## Risk owner decision

**Decision: NO-GO**, unchanged until every mandatory gate has supporting evidence. A proposed expiry of 2026-11-22 is an administrative review date, not an automatic approval or bypass.

**Approval:** [ ] Owner ____ Date ____   [ ] Security reviewer ____ Date ____

## Rollback / failure response

Do not distribute the candidate. If any startup, signature, permission, data provenance or security test fails: keep #160 blocked, retain crash diagnostics, and revert or revise the feature-gate candidate through a reviewed PR. Do not treat an unverified workaround as a hotfix or promise a production release.

**Naming check:** #215 and #216 already identify real pull requests; do not create replacement tech-debt tickets with those numbers.
