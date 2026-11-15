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

## Risk owner decision

**Decision: NO-GO**, unchanged until every mandatory gate has supporting evidence. A proposed expiry of 2026-11-22 is an administrative review date, not an automatic approval or bypass.

**Approval:** [ ] Owner ____ Date ____   [ ] Security reviewer ____ Date ____

## Rollback / failure response

Do not distribute the candidate. If any startup, signature, permission, data provenance or security test fails: keep #160 blocked, retain crash diagnostics, and revert or revise the feature-gate candidate through a reviewed PR. Do not treat an unverified workaround as a hotfix or promise a production release.

**Naming check:** #215 and #216 already identify real pull requests; do not create replacement tech-debt tickets with those numbers.
