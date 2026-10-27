# Security Policy & Hardening

## Scope
Pravda-Bot is a local-only Python application for public market surveillance. No exchange API keys, no real orders.

## Threat model
* Local execution only, dashboard bound to 127.0.0.1
* Public data sources only
* No secrets in code or logs
* Simulation paper only

## Hardening measures
* Dashboard bind 127.0.0.1 enforced in bot_config validation
* URL validation HTTPS only for NTFY_TOPIC_URL and LM_STUDIO_API_BASE
* NTFY_TOPIC_URL forced empty at startup via Lancer_Pravda_Bot.bat
* No shell=True, no eval/exec, no os.system in project code
* SQLite queries parameterized, no SQL injection surface
* .env ignored by Git, .env.example versioned
* Dependencies pinned with hashes, pip bootstrap pinned to 26.2.2
* pip-audit in CI, compileall checks

## Known risks
* pip 25.0.1 vulnerable in old venv – migrate to fresh venv with bootstrap
* SQLite at rest unencrypted – acceptable for local use, do not commit DB
* No authentication on dashboard – by design for local use only

## Reporting
Security issues: open issue with label security, no public disclosure of secrets.

## Compliance
OWASP Top 10 mitigations applied where relevant for local app.

## AIS / Maritime data legal limits
Pravda-Bot may ingest AIS data via non-commercial proxies such as aisstream.io, openwaters.io and open government feeds.

* Usage is strictly non-commercial. No redistribution of raw AIS feeds.
* Coverage is terrestrial only – no satellite data. Positions disappear over open ocean.
* Rate limits and quotas apply. Do not poll aggressively.
* Data is not verified. Positions are marked `non-commercial proxy` and must never be presented as `live_verified`.
* No commercial AIS provider is integrated. If a provider requires a paid license, integration is blocked by design.
* API keys, if used, must be stored in `.env` and never committed. Rotate keys if exposed.
* AIS data is stored in memory only by default. No persistent logging of vessel identities without explicit user consent.
* Respect local privacy and maritime regulations. Do not correlate AIS with personal data.
