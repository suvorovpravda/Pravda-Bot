# Migration Rust — point de reprise / передача / handoff

Base du lot historique : `9e910ace0091d83b68bacfd9276f39e67ed775e4` (PR #19 fusionnée).

## FR

Reprise du 31 octobre : le serveur ne réserve plus son port deux fois. La CSP utilise le port validé et autorise le WebSocket local de la page ; le navigateur réutilise l'hôte de sa page. Un test lance le vrai binaire en mode diagnostic sans flux, vérifie HTTP, WebSocket et refus d'une origine étrangère.

Lot HTTP local : middleware commun avant les routes HTTP/WebSocket, contrôle strict Host/Origin/Fetch Metadata, refus des doublons et des origines externes, en-têtes de protection sur succès et erreurs. Tests de requêtes locales, inter-origines et de rebinding sans réseau externe. SECURITY.md réécrit pour décrire le Rust actif, sans reprendre les garanties de l'archive Python.

Lot configuration : variables de processus validées pour port, base SQLite, fraîcheur, désynchronisation et plafond de spread. Les seuils alimentent réellement le garde-fou ; le port est propagé au dashboard et à sa CSP. Liaison locale conservée, réservation du port avant toute connexion marché. `.env` n'est pas chargé implicitement. Le lanceur Tauri reste à adapter au port configurable.

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
5. Renforcer CSP et débit HTTP ; contrôle Host/Origin et tests de la frontière HTTP locale désormais présents. L'authentification et la résistance à un processus local hostile ne sont pas assurées.
6. Vérifier le lanceur Tauri et produire un binaire Windows testé ; ne pas confondre workspace serveur et application desktop séparée.
7. Mesures CPU/RSS réelles et tests d'intégration sans connexion aux exchanges.
8. Étendre la configuration aux notifications et au simulateur ; contrats de parité API et tests du flux complet jusqu'aux effets. Port, chemin SQLite et seuils de marché déjà validés.

Commandes de validation :

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo +1.95.0 check --locked --workspace --all-targets
```

Aucun résultat Windows local ni test live des exchanges n'est revendiqué. Les horodatages Binance et Kraken n'ont pas la même provenance. Les seuils courants sont fixes. Les archives Python sont conservées, pas supprimées.

## RU

31 октября: исправлены двойное связывание порта и CSP; тест запускает реальный бинарный файл без биржевых подключений и проверяет HTTP, WebSocket и отклонение внешнего источника.

Добавлены проверки Host/Origin перед HTTP/WebSocket и защитные заголовки. SECURITY.md теперь описывает действующий Rust-сервер; аутентификация и ограничение нагрузки ещё отсутствуют.

Добавлена проверка настроек сервера: порт, путь SQLite и пороги рыночных данных. Порт передаётся в WebSocket и CSP; Tauri требует отдельной адаптации.

30 октября: исправлены порядок блокировок alerts/simulator, совместимость с tokio-tungstenite 0.30 и версия MSRV 1.95.0.

Добавлены JSON/CSV-маршруты истории с типизированными фильтрами и ограничением 10 000 записей. График EUR/CHF пока зависит от отсутствующего FX-коннектора.

Текущий этап исправляет рабочий Rust-сервер, но не завершает перенос всех функций. Далее нужны реальные источники энергии/новостей/FX, совместимые HTTP-маршруты, CSV, настраиваемые уведомления, безопасность HTTP и проверенная сборка Windows. Команды выше проверяют workspace сервера, а не отдельное приложение Tauri. Архив Python сохранён.

## EN

October 31: fixed the duplicate port bind and CSP. A smoke test launches the actual binary with exchange feeds disabled and checks HTTP, WebSocket and cross-origin rejection.

Host/Origin checks now precede HTTP/WebSocket routes, with defensive response headers. SECURITY.md describes the active Rust server; authentication and global load limits remain absent.

Server configuration now validates the port, SQLite path and market thresholds. The port reaches WebSocket markup and CSP; Tauri support remains separate.

October 30: repaired alerts/simulator lock ordering, tokio-tungstenite 0.30 text compatibility and MSRV 1.95.0. Toolchain-reference maintenance is now manual.

History JSON/CSV routes now support typed filters and a 10,000-row cap. EUR/CHF charts still depend on the missing FX connector.

This batch repairs the active Rust server; it does not complete feature parity. Remaining work includes real energy/news/FX sources, compatible HTTP routes, CSV reports, configurable notifications, HTTP security and a tested Windows desktop build. Commands above validate the server workspace, not the separate Tauri app. Python references remain archived.
