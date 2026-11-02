# Inventaire contractuel HTTP → IPC (RFC-0001)

**Source du relevé :** `main@423ab09c85228466d90a23269d903db3415df0c6`, fichiers `dashboard.html`, `rust/pravda-server/src/lib.rs`, `rust/pravda-server/src/auth.rs`, `rust/pravda-server/src/history.rs`.  
**Important :** cette table est un **contrat de migration proposé**, pas une liste de commandes déjà implémentées.

## Routes actives du serveur et intention IPC proposée

| Route serveur actuelle | État réel du backend | Contrat proposé pour Tauri | Auth et validation |
|---|---|---|---|
| `GET /` | HTML inclus depuis `dashboard.html` | Aucune commande : `frontendDist` Tauri | Assets intégrés, CSP |
| `GET /login` | Page/formulaire local | Écran login frontend embarqué | Non authentifié |
| `POST /login` | Mot de passe local → cookie | `auth_login(password)` | Rate limit, erreur générique, pas de secret en logs |
| `POST /setup` | Première création du vérificateur PBKDF2 | `auth_setup(password, confirm)` | Une seule configuration autorisée, atomicité |
| `POST /logout` | Révocation de cookie session | `auth_logout()` | Session courante, révocation |
| `GET /api/market` | BTC/ETH/SOL, prix, statut live/stale, temps | `market_snapshot()` | Lecture après auth, DTO typé |
| `GET /api/history` | Derniers ticks BTC/USDT plafonnés à 100 | `history_recent(limit)` | `limit` borné et symbol explicite |
| `GET /api/market/history` | Requête symbol/exchange/day-week-month, max 10 000 + `truncated` | `history_query({symbol, exchange, period})` | Enums fermés, limite, requête bornée |
| `GET /api/market/history.csv` | Export CSV | `history_export_csv(filters)` | Paramètres bornés, export sans chemin arbitraire, dialogue fichier contrôlé |
| `GET /api/alerts` | Événements réels de simulateur papier + bilan | `paper_alerts_snapshot()` | Read-only, DTO sépare alertes et bilan |
| `GET /api/alerts/check` | `{"triggered":[]}` (stub) | Aucune commande exposée avant implémentation | UI explicite indisponibilité |
| `DELETE /api/alerts/:id` | Renvoie 404 (non implémenté) | Aucune commande exposée avant implémentation | Refus par défaut |
| `GET /api/metrics` | Uptime, PID ; CPU et mémoire nuls | `runtime_metrics()` | Pas d'invention de métriques |
| `GET /api/fx` | Indisponible (`rates: {}`) | `fx_status()` sans conversion fictive | UI indique indisponibilité |
| `GET /api/news` | Réponse placeholder | Aucune commande métier avant migration | Retirer promesse de contenu |
| `GET /ws/ticks` | Flux WebSocket de ticks, session HTTP | Événements Tauri `market://ticks` / snapshots coalescés | Abonnement lié fenêtre et session, débit borné |

## Références présentes côté dashboard sans route serveur fonctionnelle

L'interface actuelle contient aussi des références `/api/sources`, `/api/eu/flows`, `/api/geopolitics`, `/api/ais/recent`, `/api/ais/geojson`, `/api/daily-report`, `/data` et un `POST /api/alerts`. Ces fonctions **ne sont pas toutes implémentées** dans le routeur Rust et ne doivent pas être converties artificiellement en commandes opérationnelles. Avant PR-04, pour chacune : décider `supprimer de l'UI`, `désactiver avec état indisponible`, ou `implémenter avec une PR fonctionnelle distincte`.

Un futur système d'alertes de seuil, d'AIS, Forex ou d'actualités doit rester hors périmètre de la simple migration IPC tant que son backend et ses contrôles ne sont pas prouvés.

## Contrats de sécurité de chaque commande

- **Entrées :** types sérialisés explicites (`serde`), `deny_unknown_fields` sur les structures sensibles, taille maximale, énumérations définies, `is_finite()` pour tout prix/calcul numérique.
- **Sorties :** DTO propres, jamais SQLite raw, mots de passe, empreintes, cookies ni chemins arbitraires.
- **Auth :** espace applicatif fermé avant `auth_setup` / `auth_login` ; une session Rust liée à la fenêtre/instance ; pas de dépendance au cookie HTTP.
- **Autorisation :** capabilities par fenêtre + manifeste des commandes + validation au niveau service Rust ; aucune passerelle générique `invoke(method, body)`.
- **Erreurs :** codes stables (ex. `NOT_CONFIGURED`, `UNAUTHORIZED`, `UNAVAILABLE`, `INVALID_INPUT`, `STORAGE_FAILURE`) et messages affichables sans données sensibles.
- **Événements :** débit et taille bornés ; risque de retard pris en compte ; état live/stale conservé ; aucun événement d'échange ne contourne le service de validation.
- **Tests :** correspondance exacte des DTO JSON existants, snapshots/équivalence, rejet des paramètres malformés, démarrage hors ligne, session expirée, callback fenêtre non autorisée.

## Point de vigilance Tauri 2

Le snippet de capability fourni comme exemple par la revue initiale **n'est pas copiable tel quel** :

- Chemin réel du projet : `crates/pravda-app/capabilities/main.json`, pas `src-tauri/capabilities`.
- Un identifiant `pravda-commands` doit correspondre à une permission véritablement déclarée/générée. Un JSON ne déclare pas, à lui seul, une permission de commande personnalisée.
- Les commandes `#[tauri::command]` doivent être explicitement verrouillées dans le build avec `tauri_build::AppManifest::commands`, en plus des capacités. Vérifier le comportement effectif en test.
- `core:webview:allow-internal-toggle-devtools` doit être absent de la Release.
- Les noms `authenticate_user`, `get_market_data`, `save_configuration` sont **illustratifs** : aucun de ces handlers Tauri n'existe encore.

Références :
https://v2.tauri.app/reference/acl/permission/
https://v2.tauri.app/security/runtime-authority/
https://v2.tauri.app/develop/plugins/
