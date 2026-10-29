# Migration Rust — point de reprise / передача / handoff

Base : `230387178413a1879774ea0b2897ebe95a218957`.

## FR

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
2. Porter les routes historique filtré/CSV, alertes configurables, sources et rapports journaliers attendues par l'interface.
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

Текущий этап исправляет рабочий Rust-сервер, но не завершает перенос всех функций. Далее нужны реальные источники энергии/новостей/FX, совместимые HTTP-маршруты, CSV, настраиваемые уведомления, безопасность HTTP и проверенная сборка Windows. Команды выше проверяют workspace сервера, а не отдельное приложение Tauri. Архив Python сохранён.

## EN

This batch repairs the active Rust server; it does not complete feature parity. Remaining work includes real energy/news/FX sources, compatible HTTP routes, CSV reports, configurable notifications, HTTP security and a tested Windows desktop build. Commands above validate the server workspace, not the separate Tauri app. Python references remain archived.
