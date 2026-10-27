# Pravda‑Bot

Pravda‑Bot est un observateur local Python pour les marchés crypto et l’énergie. Il collecte des ticks publics (Kraken/Binance), mesure les spreads, exécute une simulation papier, fournit un tableau de bord web local et expose une API d'OSINT.

## Roadmap (priorités)
- **Terminal enrichi** – historique persistant, graphiques multi‑échelle, CSV export, alertes configurables.
- **Cartographie géospatiale** – pipelines ENTSOG + OSM, AIS maritime open source, TTF prix vérifiés.
- **Veille géopolitique** – GDELT/BBC/ACLED, filtres personnalisés, journal de provenance.

## Fonctionnalités clés
* Tick collection via WebSocket (Kraken/Binance)
* Spread calculation & alerting with hysteresis
* Paper‑trading simulation (no real orders)
* SQLite persistent history (`market_history.sqlite3`)
* Local dashboard (127.0.0.1:8080) – live ticks, alerts, daily report CSV
* ENTSOG data for pipeline points (no real conduit data)
* AIS stream via open‑source APIs (aisstream.io / openwaters.io) – no commercial licence
* OSINT pipeline: GDELT, BBC RSS, GDELT news aggregation

## Installation
```bash
python -m venv .venv
.\.venv\Scripts\activate  # Windows
# or .venv/bin/activate    # Linux/Mac
pip install --require-hashes -r requirements.txt
```
Create `.env` from `.env.example` if you want to enable notifications (`NTFY_TOPIC_URL`), LM‑Studio analysis, or AIS proxy.

**Secrets**
* Ne jamais committer `.env`. `AIS_API_KEY` est affichée une seule fois sur aisstream.io/account : stocke-la uniquement dans une variable d’environnement ou un secret manager, jamais dans le code ni dans le repo.
* Le fichier `.env.example` contient les clés sans valeur.

Run:
```bash
python dashboard.py  # HTTP server
python main.py       # Bot loop
```
Open <http://127.0.0.1:8080> in your browser.

## Usage
* The bot writes ticks to `market_history.sqlite3`.
* Daily reports are exported as CSV (`daily_report.csv`).
* Alerts can be sent via ntfy if a private topic is configured.

## Security
See [SECURITY.md](SECURITY.md) for hardening guidelines, secret handling and audit results.

## Contributing
Pull requests accepted after passing tests (pytest + lint). Avoid exposing secrets in `.env` or commit history.
