<div align="center">

# 🛰️ PRAVDA-BOT
### Observatoire de marché local · Rust + Tauri 2 · Windows

**Marchés en temps réel · historique SQLite · simulation papier · interface native IPC**

[![Rust CI](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml)
[![CodeQL](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml)
![Rust 1.95+](https://img.shields.io/badge/Rust-1.95%2B-f97316?logo=rust&logoColor=white)
![Desktop](https://img.shields.io/badge/Desktop-Tauri%202-2563eb)
![Database](https://img.shields.io/badge/Database-SQLite-0e7490?logo=sqlite&logoColor=white)
![Trading](https://img.shields.io/badge/Ordres%20r%C3%A9els-AUCUN-16a34a)
![Enterprise](https://img.shields.io/badge/MSI%20sign%C3%A9-NON%20LIVR%C3%89-d97706)

**[Démarrage](#-démarrage-rapide--windows)** · **[Architecture](#-architecture-actuelle)** · **[Fonctions](#-périmètre-réel)** · **[Sécurité](SECURITY.md)** · **[Feuille de route](#-feuille-de-route)**

[Français](#français) · [Русский](#русский) · [English](#english)

</div>

---

## Français

> [!IMPORTANT]
> **État après les fusions des [PR #64](https://github.com/suvorovpravda/Pravda-Bot/pull/64) et [#67](https://github.com/suvorovpravda/Pravda-Bot/pull/67) (2 novembre 2026).** Desktop Windows Tauri 2 avec assets embarqués et **IPC natif sans serveur HTTP pour son UI**, plus serveur Axum de diagnostic séparé. La PR #67 a confirmé **8/8 checks verts et 22 assertions de sécurité IPC sous Windows WebView2 réel**. Ces vérifications portent sur **deux sessions natives successives**, pas sur deux fenêtres simultanées du même processus ni sur une origine distante. **Aucun MSI Enterprise signé n'est livré.**

### 🎯 À quoi sert Pravda-Bot ?

Pravda-Bot observe des cours **publics** Kraken/Binance, évalue leur fraîcheur et des écarts avec garde-fous, conserve un historique local et permet d'explorer une **simulation papier**.

- **Aucun ordre réel**, retrait, transaction bancaire, gestion de compte exchange ou connexion à un portefeuille.
- La simulation papier est **en mémoire** : ses résultats ne sont pas un portefeuille réellement exécuté, ni une garantie de profit.
- Les cotations live nécessitent Internet et dépendent de la disponibilité des sources. Désactiver les flux n'invente pas des prix.
- Les anciens composants Python sont conservés à titre de **référence** sous `archive/python-legacy/` ; le chemin de production Rust ne les exécute pas.

### 🏗️ Architecture actuelle

```mermaid
flowchart TB
    classDef ui fill:#dbeafe,stroke:#2563eb,color:#1e3a8a,stroke-width:2px
    classDef rust fill:#dcfce7,stroke:#16a34a,color:#14532d,stroke-width:2px
    classDef data fill:#fef3c7,stroke:#d97706,color:#78350f,stroke-width:2px
    classDef diag fill:#f3e8ff,stroke:#9333ea,color:#581c87,stroke-width:2px
    classDef external fill:#f1f5f9,stroke:#64748b,color:#334155

    subgraph DESKTOP["🪟 Windows · pravda-app.exe (Tauri 2)"]
        UI["WebView2 · HTML/CSS/JS locaux"]:::ui
        IPC["12 commandes IPC autorisées · fenêtre main"]:::ui
        AUTH["Session et mot de passe · côté Rust"]:::rust
        RUNTIME["DesktopRuntime · logique marché Rust"]:::rust
        UI -->|"invoke() · aucun port HTTP"| IPC
        IPC --> AUTH
        AUTH --> RUNTIME
    end
    subgraph STORAGE["Données locales"]
        DB[("SQLite · historique + WAL")]:::data
        PASS["Vérificateur PBKDF2 salé"]:::data
    end
    RUNTIME --> DB
    AUTH --> PASS
    EX["Kraken / Binance · flux publics"]:::external --> RUNTIME

    subgraph DIAGNOSTIC["🧪 Mode navigateur séparé"]
        BROWSER["Navigateur local"]:::diag -->|"127.0.0.1:8080"| AXUM["pravda-server · Axum"]:::diag
    end
    AXUM -.->|"mêmes composants Rust · instance séparée"| DB
```

**Deux modes, deux transports.** Le **desktop** charge `crates/pravda-app/frontend/` via `WebviewUrl::App`, utilise des commandes IPC typées et ne démarre **ni Axum ni WebSocket HTTP local**. Le **serveur de diagnostic** reste démarrable indépendamment avec `cargo run -p pravda-server`, écoute exclusivement sur `127.0.0.1` et sert une interface navigateur avec authentification HTTP. Ne pas faire tourner les deux modes simultanément sur les mêmes fichiers de données pour les tests.

Pour les décisions et contrats détaillés : [architecture](docs/architecture/README.md), [Phase 2 IPC](docs/architecture/tauri-ipc-phase2.md), [menaces](docs/architecture/threat-model.md), [migration SQLite](docs/architecture/sqlite-legacy-migration.md).

### 🪟 Démarrage rapide — Windows

**Prérequis :** Windows x64 et WebView2, [Rust/Cargo](https://rustup.rs/) (outillage Windows MSVC), prérequis C++ requis par Tauri 2 et accès aux dépendances Cargo. L'application se **compile depuis les sources** ; `bundle.active=false`, donc ce parcours **ne produit pas un MSI**.

```powershell
git clone https://github.com/suvorovpravda/Pravda-Bot.git
cd Pravda-Bot

# Vérifications du cœur Rust (workspace racine)
cargo test --locked --workspace

# Desktop natif : manifeste Cargo distinct
cargo run --release --locked --manifest-path .\crates\pravda-app\Cargo.toml
```

**Premier lancement :** création d'un mot de passe local d'au moins **15 caractères** → dashboard. Les lancements suivants demandent le mot de passe existant. Déconnexion et reconnexion restent dans la fenêtre native.

**Données Windows par défaut :**

| Fichier | Chemin | Rôle |
|:--|:--|:--|
| SQLite | `%LOCALAPPDATA%\Pravda-Bot\pravda.db` | Ticks/historique et index |
| SQLite WAL | même dossier, suffixe `-wal` / `-shm` si présents | Journaux transactionnels |
| Authentification | `%LOCALAPPDATA%\Pravda-Bot\pravda_auth.json` | Sel et vérificateur de mot de passe, **pas** le mot de passe en clair |

Les variables `PRAVDA_DB_PATH` et `PRAVDA_AUTH_PATH` prennent le dessus si elles sont définies. **N'effacez pas ces fichiers pour résoudre une erreur de connexion.** Pour une migration d'ancienne installation, sauvegarder de façon cohérente SQLite/WAL et le fichier auth ; voir [migration legacy](docs/architecture/sqlite-legacy-migration.md).

<details>
<summary><strong>🧪 Lancer un profil de test isolé, sans toucher aux données habituelles</strong></summary>

Depuis la racine du dépôt, dans **la même fenêtre PowerShell** :

```powershell
$test = Join-Path $env:TEMP ("Pravda-IPC-Test-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
New-Item -ItemType Directory -Path $test | Out-Null
$env:PRAVDA_DB_PATH = Join-Path $test "pravda.db"
$env:PRAVDA_AUTH_PATH = Join-Path $test "pravda_auth.json"
$env:PRAVDA_MARKET_FEEDS_ENABLED = "false"
Remove-Item Env:PRAVDA_ADMIN_PASSWORD -ErrorAction SilentlyContinue
Remove-Item Env:PRAVDA_LEGACY_DB_PATH -ErrorAction SilentlyContinue
Remove-Item Env:MARKET_HISTORY_DB -ErrorAction SilentlyContinue
cargo run --release --locked --manifest-path .\crates\pravda-app\Cargo.toml
```

Pour tester la **persistance**, fermer l'application puis relancer la dernière commande **sans recréer `$test` ni changer les variables**. Le mot de passe déjà créé doit rester valable. Les variables d'environnement définies dans une fenêtre PowerShell ne sont pas automatiquement visibles depuis une autre.

</details>

<details>
<summary><strong>🌐 Mode diagnostic navigateur (distinct du desktop)</strong></summary>

```powershell
# À lancer séparément, avec ses propres fichiers de données
$env:PRAVDA_DB_PATH = Join-Path $env:TEMP "pravda-diagnostic.db"
$env:PRAVDA_AUTH_PATH = Join-Path $env:TEMP "pravda-diagnostic-auth.json"
cargo run --locked -p pravda-server
```

Puis ouvrir [http://127.0.0.1:8080](http://127.0.0.1:8080). Arrêt par `Ctrl+C`. `DASHBOARD_PORT` ne concerne **que l'écoute du serveur diagnostic** ; le desktop IPC n'utilise pas ce port pour ses vues. Ne jamais publier ce serveur par tunnel, proxy ou redirection de port.

</details>

### 📊 Périmètre réel

| Fonctionnalité | Desktop IPC | Statut / limite |
|:--|:--:|:--|
| Démarrage Windows sans port HTTP UI | ✅ | `frontendDist` local, WebView2, fenêtre `main` |
| Mot de passe, setup, login, logout | ✅ | PBKDF2 salé, session côté Rust |
| BTC / ETH / SOL et indicateurs marché | ✅ | Kraken/Binance publics ; dépendants du réseau |
| Contrôles de cours et de fraîcheur | ✅ | Prix finis positifs, 4 s et 2 s par défaut, écart brut 5 % |
| Historique SQLite récent et filtré | ✅ | Requêtes typées ; maximum **10 000** lignes retournées |
| Export historique CSV | ✅ | Texte borné, téléchargement depuis la WebView ; **vérification utilisateur de l'export à compléter** |
| Alertes d'événements du simulateur papier | ✅ | Consultation ; pas de création/suppression d'alertes personnalisées |
| Métriques runtime | 🟡 | Uptime/PID ; CPU/RSS **indisponibles** |
| Forex, énergie, géopolitique, actualités, AIS | ⏳ | Parties d'UI visibles possibles, **pas de source opérationnelle garantie** |
| Véritable trading / ordres / solde exchange | ❌ | Hors périmètre |
| MSI Enterprise et signature Authenticode | ❌ | **Non livrés** ; barrière de publication à respecter |

> [!NOTE]
> **Provenance des données :** le ticker Kraken v1 est daté à sa **réception** côté client, pas à l'heure certifiée de la transaction sur l'exchange. Binance conserve un temps événement quand fourni. Les frais de simulation sont une estimation fixe de **0,36 %** ; aucun calcul ne reproduit toutes les contraintes d'exécution réelle.

### ⚙️ Configuration et garde-fous

Les variables sont lues depuis **l'environnement du processus**, sans chargement implicite de `.env`.

| Variable | Valeur par défaut | Usage / validation |
|:--|:--|:--|
| `PRAVDA_DB_PATH` | `pravda.db` dans le serveur ; AppData dans Tauri Windows | SQLite, chemin non vide ; `:memory:` refusé |
| `PRAVDA_AUTH_PATH` | `pravda_auth.json` dans le serveur ; AppData dans Tauri Windows | Vérificateur local ; chemin non vide |
| `PRAVDA_MARKET_FEEDS_ENABLED` | `true` | `true` ou `false` exclusivement |
| `MARKET_DATA_STALE_SECONDS` | `4` | > 0 et ≤ 3 600 secondes |
| `MAX_EXCHANGE_SKEW_SECONDS` | `2` | Entre 0 et 3 600 secondes |
| `MAX_ALLOWED_SPREAD_PERCENT` | `5` | Nombre fini > 0 et ≤ 100 |
| `PRAVDA_LEGACY_DB_PATH` | Non définie | Source SQLite Python **lecture seule**, si migration demandée |
| `MARKET_HISTORY_DB` | Non définie | Chemin legacy alternatif, priorité moindre |
| `DASHBOARD_PORT` | `8080` | **Serveur navigateur uniquement** ; 1 à 65 535 |

Un paramètre invalide interrompt le démarrage ; augmenter les seuils ne résout pas la provenance incertaine d'un cours. Pour `PRAVDA_ADMIN_PASSWORD` (provisionnement optionnel), préférer le setup interactif sur un poste utilisateur et éviter l'exposition de secrets dans les commandes et les journaux.

### 🔐 Modèle de sécurité, en résumé

```mermaid
flowchart LR
    classDef allow fill:#dcfce7,stroke:#16a34a,color:#14532d
    classDef block fill:#fee2e2,stroke:#dc2626,color:#7f1d1d
    classDef step fill:#dbeafe,stroke:#2563eb,color:#1e3a8a
    LOCAL["Fenêtre Tauri main + assets embarqués"]:::step --> ACL{"Capability + fenêtre autorisées ?"}:::step
    ACL -->|non| DENY["Refus IPC"]:::block
    ACL -->|oui| AUTH{"Session Rust valide ?"}:::step
    AUTH -->|non · hors bootstrap auth| DENY
    AUTH -->|oui| READ["Snapshot / historique / CSV borné"]:::allow
```

- **Défense en profondeur :** capabilities Tauri restreintes à `main`, manifeste de commandes nommé, contrôle de session dans les handlers Rust.
- **Aucune permission IPC générale `shell` / `fs`**, aucun script distant, ressources JS/CSS locales et CSP spécifique au desktop.
- **Séparation des transports :** le desktop ne démarre pas de service HTTP ; le serveur diagnostic applique ses propres contrôles Host/Origin, cookies et en-têtes.
- **Limites non résolues :** un logiciel malveillant ayant les droits de l'utilisateur peut atteindre ses fichiers ; l'analyse de code, la CI verte et un mot de passe local **ne garantissent pas l'absence de vulnérabilités**.
- **Gouvernance solo :** le propriétaire maintient seul le dépôt et a supprimé l'obligation d'approbation d'un deuxième développeur. Les **PR, checks CI et protections de `main`** restent indispensables. L'option « no bypass » apparaît cochée dans une capture de la configuration, mais l'API de l'intégration ne peut pas confirmer les réglages sauvegardés (HTTP 403).

➡️ **Lire la [politique SECURITY.md](SECURITY.md)** pour les menaces, les mesures vérifiées, les risques restants et le signalement privé.

### 🧪 Tests et vérification

```powershell
# Workspace Rust du serveur : Linux et Windows
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --release --workspace

# Le desktop Tauri est un workspace Cargo distinct
cargo fmt --manifest-path .\crates\pravda-app\Cargo.toml -- --check
cargo test --locked --manifest-path .\crates\pravda-app\Cargo.toml
cargo build --locked --manifest-path .\crates\pravda-app\Cargo.toml

# Contrats JavaScript statiques (Node.js, idéalement v22)
node tools/test-dashboard.mjs
node tools/test-desktop-ipc.mjs
```

**CI GitHub :** Rust Linux/Windows, MSRV 1.95, smoke Windows desktop sans listener, serveur navigateur, CodeQL. Le badge est dynamique : un commit fusionné demande une **nouvelle vérification des jobs de `main`**, pas seulement des checks verts sur la PR.

**Recette Windows effectuée après Phase 2 :** lancement natif, setup/login, refus d'un mauvais mot de passe, logout/relogin, redémarrage, mode sans live, fichiers SQLite/auth et absence de socket TCP en écoute pour `pravda-app.exe`. **PR #67 :** 22 tests E2E natifs WebView2 réussis, dont refus `UNAUTHORIZED` sans session/après logout, refus des IPC depuis `security-probe` et blocage shell/fs avec absence d'écriture. **Restent non démontrés :** origine distante, deux fenêtres simultanées avec session partagée, export CSV réel, reprise historique complexe et distribution signée.

### 🗺️ Feuille de route

```mermaid
flowchart LR
    classDef done fill:#dcfce7,stroke:#16a34a,color:#14532d
    classDef next fill:#dbeafe,stroke:#2563eb,color:#1e3a8a
    classDef pending fill:#ffedd5,stroke:#ea580c,color:#7c2d12
    A["PR-01 · Contrats, architecture, menaces"]:::done --> B["SQLite · Import legacy transactionnel"]:::done
    B --> C["PR #64 · Desktop Tauri IPC intégré"]:::done
    C --> D["PR #67 · IPC natif testé sous Windows"]:::done
    D --> E["Parité · FX/énergie/news/AIS, alertes"]:::pending
    E --> F["Distribution · MSI signé, SBOM, rollback"]:::pending
```

| Priorité | Chantier | Critère de sortie |
|:--|:--|:--|
| **P0** | Gouvernance solo et checks post-fusion | PR/checks/no-bypass maintenus ; confirmation des réglages enregistrés par le propriétaire ; E2E permanent dans la CI |
| **P0/P1** | Étendre la recette IPC native | Les **22 assertions Windows** de #67 sont acquises ; ajouter tests origines distantes et fenêtres simultanées en processus unique |
| **P1** | Compléter la parité Rust | Sources avec provenance vérifiée, UI affichant « indisponible » si absent |
| **P1** | Qualité des données et persistance | Rejeu/migration SQLite, CSV borné, tests redémarrage/rollback |
| **P2** | Packaging Enterprise | MSI/EXE **signés et horodatés**, SBOM et intégrité CI, tests sur Windows propre |
| **P2** | Observabilité et ergonomie | CPU/RSS mesurés, erreurs redigées, diagnostics compatibles Windows |

**Pas de date de livraison inventée.** Une fonctionnalité reste « prévue » tant que ses tests et preuves de sortie ne sont pas intégrés. Voir le [suivi de migration](MIGRATION_STATUS.md) et les [critères de release](docs/architecture/acceptance.md).

### 🤝 Contribuer et obtenir de l'aide

1. Créer une branche dédiée, garder les modifications atomiques et ajouter des tests.
2. Exécuter les commandes adaptées au **workspace Rust** et au **desktop Tauri**.
3. Ouvrir une PR, faire revoir le modèle de menace/capabilities si une permission IPC change.
4. Contrôler les checks **du SHA exact**, documenter les essais Windows et obtenir les approbations requises avant fusion.

[Issues](https://github.com/suvorovpravda/Pravda-Bot/issues) · [Pull requests](https://github.com/suvorovpravda/Pravda-Bot/pulls) · [Sécurité privée](SECURITY.md#-signaler-une-vulnérabilité)

---

## Русский

**Pravda-Bot** — локальный монитор публичных котировок Kraken/Binance, истории SQLite и **бумажной симуляции**, без реальных торговых ордеров и доступа к счетам. После [PR #64](https://github.com/suvorovpravda/Pravda-Bot/pull/64) Windows-приложение Tauri 2 использует **встроенный интерфейс и IPC** без локального HTTP-сервера для UI. `pravda-server` (Axum, `127.0.0.1:8080`) существует отдельно для диагностики.

**Windows:** `cargo run --release --locked --manifest-path .\crates\pravda-app\Cargo.toml`. При первом запуске требуется пароль от 15 символов; данные по умолчанию находятся в `%LOCALAPPDATA%\Pravda-Bot`. Для отключения биржевых потоков установите `PRAVDA_MARKET_FEEDS_ENABLED=false`. Firefox/Chrome не нужны для Tauri, но WebView2 необходим.

**Ограничения:** FX, энергетика, новости и AIS реализованы не полностью. Готового подписанного Enterprise MSI нет; зелёная CI не подтверждает безопасность выпуска. Полные инструкции, таблица переменных и план находятся во [французском разделе](#français) и [SECURITY.md](SECURITY.md).

## English

**Pravda-Bot** is a local observer of public Kraken/Binance prices with SQLite history and **paper-only simulation**. No real orders, account access or real portfolio execution. Since [PR #64](https://github.com/suvorovpravda/Pravda-Bot/pull/64), the Windows Tauri 2 desktop loads **bundled assets and typed IPC** with **no local HTTP listener for its UI**. The Axum `pravda-server` remains a **separate diagnostic CLI**, loopback-bound.

**Build/run on Windows:** `cargo run --release --locked --manifest-path .\crates\pravda-app\Cargo.toml`. Create a local password of at least 15 characters on first run. Default data storage: `%LOCALAPPDATA%\Pravda-Bot`. To disable feeds for testing, set `PRAVDA_MARKET_FEEDS_ENABLED=false`.

**Current limitations:** FX, energy, news, AIS and customizable alerts are not fully migrated. No signed Enterprise MSI has been delivered. CI and manual Windows acceptance are meaningful evidence, **not a security certification**. See [the full French guide](#français), [SECURITY.md](SECURITY.md) and the [roadmap](#-feuille-de-route).
