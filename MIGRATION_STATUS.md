# Migration Rust — point de reprise / передача / handoff

Base du lot historique : `9e910ace0091d83b68bacfd9276f39e67ed775e4` (PR #19 fusionnée).

## FR

Reprise du 30 octobre : correction de l'ordre des verrous de `/api/alerts` (test déterministe échouant avant correction), adaptation des messages texte à tokio-tungstenite 0.30, restauration de la MSRV 1.95.0. Dependabot ne modifie plus automatiquement la référence rust-toolchain ; sa maintenance reste manuelle.

Lot historique : `/api/market/history` et `/api/market/history.csv` raccordés en Rust. Filtres typés symbol/exchange/period, ordre chronologique, limite 10 000, troncature explicite, requêtes SQLite paramétrées et indexées, erreurs 503 sans détails internes. Tests HTTP sans réseau externe. Le graphique EUR/CHF dépend encore du connecteur FX non migré ; ces routes ne suffisent pas à rendre toutes les courbes visibles.

Lot courant : réparer le chemin Rust réellement exécuté avant de poursuivre la parité fonctionnelle.

- Parseur Kraken v1 : tableau ticker réel, USDT exclusivement, variantes BTC/XBT.
- Binance : temps événement conservé, prix invalides refusés.
- Horloge monotone transportée avec chaque tick ; contrôle après persistance et avant simulation.
- Calcul `pravda-market-core` raccordé au serveur : frais en fraction et plafond 5 %.
- SQLite déplacé vers `spawn_blocking` ; erreurs journalisées sans détails sensibles.
- Retard broadcast : traitement poursuivi, anciennes cotations et confirmations invalidées.
- Tâches WebSocket fermées ensemble à la déconnexion.
- Simulation : conservation du signe lors d'un renversement de spread, rejet des nombres invalides.
- Uptime réel ; CPU/mémoire `null`. Ce lot n'implémente pas leur collecte.
- Lockfile régénéré ; TLS WebSocket via Rustls ; MSRV CI corrigée à 1.95.0.

À reprendre, sans annoncer une migration complète :

1. Remplacer le reste des réponses provisoires énergie/actualités/FX par des adaptateurs testés avec provenance et indisponibilité explicite.
2. Porter les alertes configurables, sources et rapports journaliers ; historique filtré JSON/CSV désormais implémenté.
3. Porter les estimations sur carnets, exports et outils d'analyse hors ligne ; comparer les résultats de référence aux archives.
4. Porter ntfy/LLM optionnels avec secrets masqués et revalidation des snapshots avant chaque effet.
5. Renforcer CSP, contrôle Host/Origin, débit HTTP et tests de sécurité du nouveau serveur.
6. Vérifier le lanceur Tauri et produire un binaire Windows testé ; ne pas confondre workspace serveur et application desktop séparée.
7. Mesures CPU/RSS réelles et tests d'intégration sans connexion aux exchanges.
8. Configuration validée des seuils, du port et des chemins ; contrats de parité API et tests du flux complet jusqu'aux effets.

Commandes de validation :

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo +1.95.0 check --locked --workspace --all-targets
```

Aucun résultat Windows local ni test live des exchanges n'est revendiqué. Les horodatages Binance et Kraken n'ont pas la même provenance. Les seuils courants sont fixes. Les archives Python sont conservées, pas supprimées.

## RU

30 октября: исправлены порядок блокировок alerts/simulator, совместимость с tokio-tungstenite 0.30 и версия MSRV 1.95.0.

Добавлены JSON/CSV-маршруты истории с типизированными фильтрами и ограничением 10 000 записей. График EUR/CHF пока зависит от отсутствующего FX-коннектора.

Текущий этап исправляет рабочий Rust-сервер, но не завершает перенос всех функций. Далее нужны реальные источники энергии/новостей/FX, совместимые HTTP-маршруты, CSV, настраиваемые уведомления, безопасность HTTP и проверенная сборка Windows. Команды выше проверяют workspace сервера, а не отдельное приложение Tauri. Архив Python сохранён.

## EN

October 30: repaired alerts/simulator lock ordering, tokio-tungstenite 0.30 text compatibility and MSRV 1.95.0. Toolchain-reference maintenance is now manual.

History JSON/CSV routes now support typed filters and a 10,000-row cap. EUR/CHF charts still depend on the missing FX connector.

This batch repairs the active Rust server; it does not complete feature parity. Remaining work includes real energy/news/FX sources, compatible HTTP routes, CSV reports, configurable notifications, HTTP security and a tested Windows desktop build. Commands above validate the server workspace, not the separate Tauri app. Python references remain archived.
