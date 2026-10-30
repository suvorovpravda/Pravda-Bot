<div align="center">

# Pravda-Bot · Rust

Surveillance des marchés · Simulation papier · Données locales

[![Rust CI](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml)

[FR](#français) · [RU](#русский) · [EN](#english)

</div>

## Français

Pravda-Bot est un observatoire local des prix publics Kraken/Binance. Il sert à étudier les écarts, conserver des ticks dans SQLite et expérimenter une simulation papier. **Il ne passe aucun ordre et ne vérifie aucun compte d'exchange.**

Le serveur actif est en Rust (Tokio/Axum). Les anciennes implémentations Python sont conservées dans `archive/python-legacy/` comme références : elles ne sont pas exécutées par le serveur Rust. La migration fonctionnelle complète reste en cours.

### Démarrage

Installer Rust avec Cargo, puis depuis le dépôt :

```sh
cargo test --locked --workspace
cargo run --locked -p pravda-server
```

Ouvrir http://127.0.0.1:8080. Le serveur écoute uniquement la boucle locale et crée `pravda.db` dans le répertoire de lancement. Arrêt : Ctrl+C. Il nécessite un accès Internet pour les flux publics ; local ne signifie pas hors ligne. Le serveur n'a pas besoin de Python.

### Configuration du serveur Rust

Définir les variables dans l'environnement du processus avant le lancement. Le serveur ne charge pas automatiquement les fichiers `.env` de l'ancienne version.

| Variable | Défaut | Validation |
|---|---|---|
| `DASHBOARD_PORT` | 8080 | Entier de 1 à 65535 ; écoute toujours sur 127.0.0.1 |
| `PRAVDA_DB_PATH` | pravda.db | Chemin non vide, sans octet nul ; base mémoire refusée ; dossier parent existant |
| `MARKET_DATA_STALE_SECONDS` | 4 | Durée strictement positive, au plus 3600 secondes |
| `MAX_EXCHANGE_SKEW_SECONDS` | 2 | Durée de 0 à 3600 secondes |
| `MAX_ALLOWED_SPREAD_PERCENT` | 5 | Valeur finie strictement positive, au plus 100 points de pourcentage |

Sous PowerShell, `$env:DASHBOARD_PORT='8081'` puis `cargo run --locked -p pravda-server` utilise http://127.0.0.1:8081. Une configuration invalide ou un port occupé interrompt le démarrage avant les connexions marché. Les frais restent fixés à 0,36 %. Augmenter les limites de fraîcheur ou d'anomalie réduit la protection. Le lanceur Tauri séparé attend encore 8080 : ces options sont actuellement destinées au serveur en ligne de commande.

### Fonctionnement et limites

| Composant | État réel |
|---|---|
| Réception | Binance ticker avec temps événement ; Kraken v1 ticker avec temps de réception uniquement |
| Contrôles | Prix finis positifs, fraîcheur 4 s, désynchronisation 2 s, plafond d'écart brut 5 % |
| Frais | Estimation fixe de 0,0036 en fraction, soit 0,36 point de pourcentage |
| Simulation | Moteur Rust en mémoire ; aucun ordre, carnet exécuté, solde réel ou transfert |
| Historique | SQLite WAL ; JSON/CSV filtrés BTC/ETH/SOL, Kraken/Binance, jour/semaine/mois ; 10 000 ticks maximum avec indicateur de troncature |
| Métriques | Uptime et PID réels ; CPU et mémoire indisponibles (`null`) |
| Interface | HTML/JavaScript servi par Rust ; certains panneaux de l'ancienne interface restent à raccorder |
| Énergie, actualités, Forex, AIS | Migration non terminée : aucune promesse de données opérationnelles |
| Desktop Tauri | Projet séparé, non couvert par les tests du workspace serveur ; build Windows à vérifier |

Un ticker Kraken v1 est actualisé à chaque transaction ; une absence de transaction peut produire un état STALE. Son horodatage local ne prouve pas l'âge de la transaction à l'exchange. Le seuil ne doit pas être élargi pour cacher ce manque d'information.

L'interface utilise encore des scripts inline. La liaison locale ne remplace pas une authentification : ne pas exposer le port par un proxy public. Les résultats papier ne modélisent pas les contraintes de remplissage, marge, emprunt, inventaire ou latence de placement.

Le serveur contrôle désormais `Host`, `Origin` et le contexte navigateur avant les routes HTTP/WebSocket. Les requêtes vers un hôte ou un port inattendu sont refusées. Les clients locaux sans `Origin` restent utilisables ; ce contrôle ne protège pas contre un programme malveillant exécuté sur le poste. Voir [SECURITY.md](SECURITY.md).

### Contribuer et reprendre le travail

Chaque lot doit avoir des tests Rust, un commit identifiable et une CI Windows/Linux vérifiée avant fusion. Ne pas présenter un module isolé comme intégré. Les archives ne sont pas une suite de validation de la nouvelle application. Consulter [MIGRATION_STATUS.md](MIGRATION_STATUS.md) pour les travaux restants et les commandes de contrôle.

Ces composants peuvent servir à construire d'autres observatoires locaux, outils de qualité des données et simulateurs reproductibles. Les contributions prioritaires sont la parité des routes HTTP, les tests d'intégration, les sources avec provenance et le packaging Windows.

Disponibilité annoncée par le responsable : la moitié de l'équipe est en vacances, reprise prévue vers le 2 novembre 2026.

## Русский

Rust-сервер читает переменные окружения процесса, но не загружает `.env` автоматически. Порт задаётся `DASHBOARD_PORT`, база — `PRAVDA_DB_PATH`, ограничения — переменными из таблицы выше. Ошибочные значения останавливают запуск. Отдельный Tauri-клиент пока ожидает порт 8080.

Pravda-Bot — локальное приложение наблюдения за публичными ценами Kraken/Binance. Сервер написан на Rust (Tokio/Axum), история хранится в SQLite. Реальные ордера не отправляются; счета и остатки не проверяются. Старый Python-код находится в `archive/python-legacy/` и не запускается Rust-сервером.

Запуск: `cargo test --locked --workspace`, затем `cargo run --locked -p pravda-server`. Интерфейс: http://127.0.0.1:8080. Для рыночных данных требуется Интернет.

Полный перенос функций ещё не завершён: энергетика, новости, валюты, AIS, часть HTTP-маршрутов и приложение Tauri требуют дальнейшей работы. CPU и память возвращаются как недоступные, а не как нулевые измерения. Данные Kraken v1 имеют локальное время получения, но не подтверждённое время сделки. Симуляция не отражает реальное исполнение.

Каждое изменение проходит тесты и отдельный коммит; перед слиянием требуется CI Windows/Linux. План: [MIGRATION_STATUS.md](MIGRATION_STATUS.md). По информации руководителя, половина команды в отпуске, возвращение ожидается около 2 ноября 2026 года.

## English

The Rust server reads process environment variables; it does not automatically load `.env`. Configure the port with `DASHBOARD_PORT`, storage with `PRAVDA_DB_PATH`, and guard thresholds using the table above. Invalid values stop startup. The separate Tauri client still expects port 8080.

Pravda-Bot observes public Kraken/Binance prices locally. Its active server uses Rust, Tokio/Axum and SQLite. It sends no real orders and verifies no exchange accounts. Python sources under `archive/python-legacy/` are reference material, not part of the Rust runtime.

Run `cargo test --locked --workspace`, then `cargo run --locked -p pravda-server`. Open http://127.0.0.1:8080. Public market feeds require Internet access.

Functional migration is incomplete: energy, news, FX, AIS, several dashboard routes and the separate Tauri desktop build need further work. CPU and memory are explicitly unavailable, not fabricated zero measurements. Kraken v1 reception time does not establish exchange trade age. Paper results do not model real fills or account constraints.

Changes require tests, identifiable commits and Windows/Linux CI before merging. See [MIGRATION_STATUS.md](MIGRATION_STATUS.md). The project owner reports that half the team is on holiday, returning around November 2, 2026.
