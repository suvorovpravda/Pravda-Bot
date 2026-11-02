# Phase 2 — Desktop Tauri IPC, assets embarqués

**État : PR brouillon non livrable sans revue/validation Windows réelle.** Référence : issue #61.

## Architecture cible de cette PR

- La fenêtre `main` charge `WebviewUrl::App("index.html")` depuis `build.frontendDist: "frontend"`.
- `styles.css`, `auth.js` et `dashboard.js` sont des ressources locales. Le dashboard HTML/CSS existant est préservé, séparé du JavaScript inline.
- Le JavaScript utilise exclusivement `window.__TAURI__.core.invoke` (bundle Tauri) ; aucun HTTP/WS local n'est ouvert pour l'UI. Le démarrage ne dépend d'aucune sonde HTTP ni de port `DASHBOARD_PORT`.
- `pravda_server::DesktopRuntime` démarre les flux et partage `PravdaRuntime`, `PravdaStore` et la migration legacy. `pravda-server` Axum reste un programme CLI de diagnostic séparé.
- La session autorisée demeure possédée côté Rust ; aucune session token/cookie n'est envoyée au JavaScript. `ipc_auth_status/setup/login` sont des commandes de bootstrap avant connexion, tandis que toute commande métier et `ipc_auth_logout` vérifient `require_authenticated()`.
- Les filtres history `HistoryQuery` reprennent les enums et plafonds historiques ; l'export CSV renvoie du texte borné, sans exposer l'écriture de fichiers arbitraires à l'IPC.
- Capabilities `windows: ["main"]` uniquement, manifeste explicite des 12 commandes, aucune origine distante, aucune permission générale shell/fs. CSP scripts et styles locaux ; `connect-src` limité aux schémas du pont IPC `ipc:` et `http://ipc.localhost` (pas de serveur HTTP lancé par l'application).

## Correspondance des routes

| Interface HTTP historique | Commande IPC |
| --- | --- |
| `/login`, `/setup`, `/logout` | `ipc_auth_status/setup/login/logout` |
| `/api/market` | `ipc_market_snapshot` |
| `/api/history` | `ipc_history_recent` |
| `/api/market/history` | `ipc_history_query` |
| `/api/market/history.csv` | `ipc_history_export_csv` (Blob local) |
| `/api/alerts` | `ipc_paper_alerts_snapshot` |
| `/api/metrics` | `ipc_runtime_metrics` |
| `/api/fx` | `ipc_fx_status` (*indisponible* véridique) |
| `/ws/ticks` | Polling `ipc_market_snapshot` chaque seconde, sans WebSocket |

Les références présentes dans le dashboard à `/api/sources`, `/api/geopolitics`, `/api/ais/*`, `/api/eu/flows`, `/api/daily-report`, suppression/création d'alertes et autres stubs sont explicitement **indisponibles**. Aucune simulation de données n'a été ajoutée.

## Tests et gates

Automatique : `node tools/test-desktop-ipc.mjs`, `node tools/test-dashboard.mjs` (parité serveur navigateur), `cargo fmt`, `cargo clippy`, `cargo test`, release sur Windows/Linux, MSRV, CodeQL, build desktop Windows `--locked` et smoke Windows **sans socket HTTP**. Les tests navigateur Axum indépendants restent exécutés par `browser-fallback.yml`.

**Manuel avant fusion (impératif)** : Windows 11 natif, WebView2 réel, login ancien `pravda_auth.json`, setup nouveau profil, logout, re-login, refus IPC hors session, fenêtre non `main`, origine distante, redémarrage sur historique SQLite legacy, rafraîchissement BTC/ETH/SOL sans réseau et avec réseau, CSV borné et affichage indisponible pour les fonctions non migrées. Vérifier `Get-NetTCPConnection` et processus : aucun listener HTTP du desktop ni serveur enfant. Vérifier le fichier `Cargo.lock` imbriqué inchangé et des logs sans secrets.

## Risques et rollback

L'interface n'a plus de fallback HTTP : un problème de CSP ou de capability doit afficher une erreur, pas rouvrir le port 8080. Sauvegarder les deux fichiers SQLite/WAL et le vérificateur `pravda_auth.json` avant tout essai utilisateur. Le rollback est la restauration de l'exécutable Tauri de la release précédente : **ne pas** effacer les données utilisateur et ne pas réverter la migration SQLite.

**Bloqueur administratif séparé :** protections de `main` (PR obligatoire, approbation humaine, checks requis, sans bypass) et revue humaine de la PR. **Aucune fusion d'un brouillon.**
