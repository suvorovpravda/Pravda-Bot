# Pravda-Bot v0.2 — migration Rust en cours

## Français
### Ce qui change
* Migration partielle vers Rust : module `rust/market-core` avec calcul de spread, validation des prix/timestamps, contrôle de fraîcheur et persistance SQLite expérimentale isolée.
* Persistance Rust : `PRAGMA journal_mode=WAL`, `busy_timeout` configurable, pool r2d2 à 4 connexions, création de base isolée, refus d’ouvrir une base existante.
* Stabilisation Windows SQLite : `tearDown()` avec `gc.collect()` dans `tests/test_market_history_contention.py` pour éviter les `PermissionError` de nettoyage.
* Dashboard : vérification CSP sur scripts inline, présence de `checkPriceAlerts`, pas de null-byte ni espaces insécables dans `dashboard.html`.
* Documentation : section Architecture Local-First Rust avec diagramme Mermaid, guide `cargo build --release`, mise à jour de `SECURITY.md`.

### Limites explicites
* Le module Rust n’est pas encore raccordé au bot en fonctionnement. Simulation papier uniquement, aucun ordre réel.
* Dashboard local sans authentification, liaison stricte 127.0.0.1.
* Pas de publication PyPI dans cette release.

### Comment vérifier
```bash
python -m unittest discover -s tests -t . -q
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

## Русский
### Что изменилось
* Частичная миграция на Rust: модуль `rust/market-core` с расчётом спреда, валидацией цен/временных меток, контролем свежести и экспериментальным изолированным SQLite.
* Rust-персистентность: `PRAGMA journal_mode=WAL`, настраиваемый `busy_timeout`, пул r2d2 из 4 соединений, создание изолированной базы, отказ открывать существующую.
* Стабилизация Windows SQLite: `tearDown()` с `gc.collect()` в тестах contention для закрытия handles.
* Dashboard: проверка CSP для inline-скриптов, наличие `checkPriceAlerts`, отсутствие null-байтов.
* Документация: раздел Local-First Rust с Mermaid, руководство `cargo build --release`, обновление `SECURITY.md`.

### Ограничения
* Модуль Rust ещё не подключён к рабочему боту. Только бумажная симуляция, реальные ордера не исполняются.
* Локальный дашборд без аутентификации, привязка к 127.0.0.1.
* Публикация на PyPI отсутствует.

## English
### What changed
* Partial Rust migration: `rust/market-core` with spread calculation, price/timestamp validation, freshness control and isolated experimental SQLite persistence.
* Rust persistence: `PRAGMA journal_mode=WAL`, configurable `busy_timeout`, r2d2 pool of 4 connections, isolated DB creation, refuses to open existing DB.
* Windows SQLite stabilization: `tearDown()` with `gc.collect()` in contention tests to avoid file lock `PermissionError`.
* Dashboard: CSP validation on inline scripts, `checkPriceAlerts` present, no null bytes in `dashboard.html`.
* Documentation: Local-First Rust architecture section with Mermaid diagram, `cargo build --release` guide, `SECURITY.md` update.

### Explicit limits
* Rust module not yet wired to the running bot. Paper simulation only, no real orders.
* Local dashboard without authentication, bound to 127.0.0.1.
* No PyPI publication in this release.

**Tag cible** : `v0.2` sur `rust/market-core-migration`  
**SHA de référence** : `2b91ec1`  
**PR brouillon** : #6
