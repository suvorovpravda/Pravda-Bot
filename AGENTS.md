## Codex task-boundary board

- This repository uses the opt-in Codex task-boundary board in `.codex/coordination/project.yaml`.
- Before substantial writes, load the installed `codex-coordinator` skill, list active claims from the primary worktree, and publish only this task's bounded claim.
- Native Codex tasks remain the execution, messaging, and transcript authority; an explicitly requested goal Coordinator is on demand, with no heartbeat or mandatory pull-request workflow.
- Reject cross-project notices and never store transcripts, reasoning, prompts, or tool output in Coordinator state.

## Pravda security and release restrictions

- Keep issue #160 NO-GO in force: PAPER only, no LIVE trading, no production deploy, no signing or release without explicit authorization and independent evidence.
- Do not merge pull requests automatically or waive quality/security gates.
- Coordinate substantial tasks by bounded file ownership; do not create background monitoring or recurring agents.
