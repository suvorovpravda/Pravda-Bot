# CHANGE / REVIEW

**Scope:** <!-- one sentence describing the real user-visible or security issue -->

## What changed
<!-- Exact files/modules and effects. Note any new permissions, dependency or network access. -->

## Why it is safe
- [ ] No credentials, personal data or keys in frontend, IPC, reports, logs or CI.
- [ ] No unexpected network connections, financial transactions or privilege elevation.
- [ ] Rust session/authorization boundary remains intact.
- [ ] SQLite state is preserved; no implicit reset or migration without proof.

## Evidence on this exact commit
**Head SHA:** <!-- 40-digit Git commit -->

| Control | Result | Run/job/evidence |
| :--- | :--- | :--- |
| Rust format, clippy, Windows/Linux tests | NOT RUN | |
| CodeQL and native IPC/WebView2 tests | NOT RUN | |
| Desktop RustSec scan | NOT RUN | |
| Windows GUI startup, first login and restart | NOT RUN | |
| MSI and NSIS installed smoke test | NOT RUN | |
| Diagnostic report / privacy | NOT RUN | |
| Artifact hash and signature policy | NOT RUN | |

## Risks, SKIPs and open blockers
<!-- Distinguish unit tests, synthetic market fixtures, and observed live public ticks.
     Any failed/unfinished release gate is NO-GO. Reference issue #160. -->

## Review decision
- [ ] READY FOR REVIEW
- [ ] CHANGES REQUESTED
- [ ] APPROVED FOR MERGE
- [ ] NO-GO DISTRIBUTION

**Release policy:** See [REVIEWING](../docs/REVIEWING.md), [SECURITY](../SECURITY.md) and [Security / Deployment issue #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160). Closing this PR does not imply shipping approval.
