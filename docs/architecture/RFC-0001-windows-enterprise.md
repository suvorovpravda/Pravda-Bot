# RFC-0001 — Refonte du desktop Windows vers Rust/Tauri IPC

**Statut :** proposé pour revue  
**Date :** 2026-11-02  
**Base d'analyse :** `423ab09c85228466d90a23269d903db3415df0c6`  
**Décision cible :** maintenir Rust, Tauri 2, SQLite et l'UI HTML/CSS/JS ; éliminer le serveur HTTP embarqué comme transport desktop ; livrer un MSI signé.

## 1. Contexte vérifié

- `crates/pravda-app/src/main.rs` initialise les chemins de données locaux, teste le port, démarre `pravda_server::run_server()` et construit une `WebviewUrl::External(http://127.0.0.1:8080)` après une sonde HTTP.
- `crates/pravda-app/tauri.conf.json` a `bundle.active=false`, `app.windows=[]`; aucune politique de capacités métier n'est définie.
- `rust/pravda-server/src/lib.rs` contient le moteur temps réel et les handlers Axum dans une seule unité de déploiement ; `dashboard.html` utilise `fetch` et `WebSocket`.
- `rust/pravda-server/src/auth.rs` définit un mot de passe >=15 caractères, PBKDF2-SHA256 avec sel et 600 000 itérations, enregistrement local, cookies de session ; le parcours desktop utilise des formulaires HTTP.
- `crates/pravda-store` ouvre SQLite en WAL ; `crates/pravda-fetchers` reçoit Kraken/Binance ; `crates/pravda-engine` exécute le simulateur papier.
- Des fichiers de secours `tools/*.cmd`, `*.ps1` utilisent PowerShell, notamment `-ExecutionPolicy Bypass`. **Le binaire Tauri actuel ne les appelle pas** ; ils ne doivent toutefois pas entrer dans le paquet de production.
- Échecs Windows observés par un utilisateur : navigation `request origin not allowed`, erreur de diagnostic de chemin, disparition après saisie de mot de passe. La cause complète sur ce poste **n'est pas encore établie**.

## 2. Cible logique

```text
Pravda-Bot.exe (Rust/Tauri Windows GUI process)
  ├─ Tauri WebView2, assets HTML/CSS/JS EMBARQUÉS, CSP restrictive
  ├─ ACL Tauri par fenêtre + manifeste de commandes personnalisées
  ├─ Typed IPC commands [requires auth + per-command authorization]
  ├─ Rust application-service layer
  │    ├─ Auth + session lifecycle
  │    ├─ Market snapshots / event fan-out (bounded)
  │    ├─ Paper simulation + alerts
  │    └─ SQLite store (single owner, bounded reads)
  └─ Stable user-data directory (%LOCALAPPDATA%/Pravda-Bot)
       ├─ SQLite WAL database
       ├─ password verifier (non-reversible)
       └─ redacted rotating diagnostics
Separate out-of-product target: pravda-server.exe (Axum, CLI diagnostic only)
```

Le desktop ne doit ni lancer un navigateur externe, ni appeler des scripts, ni ouvrir une écoute sur `127.0.0.1`. Le serveur diagnostic reste testable depuis un binaire distinct mais ne figure pas dans MSI/paquet utilisateur. Les tests desktop doivent vérifier l'absence d'écoute par l'application elle-même, sans affirmer qu'aucun autre service ne peut utiliser le même port.

## 3. Décisions de conception

**Services Rust d'abord.** Extraire les mutations et lectures d'AppState en services sans dépendance `axum::extract` ; le serveur CLI et les commandes Tauri deviennent deux adaptateurs sur la même logique. Ne pas convertir tous les handlers HTTP directement en `#[tauri::command]`.

**IPC typé, non générique.** Proposer une commande par intention (ex. `market_snapshot`, `history_query`, `auth_setup`, `auth_login`, `auth_logout`), avec `serde` strict, limites d'entrée, `Result<_, SafeError>` et réponse versionnée. Pas d'IPC donnant accès à un chemin libre, `shell`, SQL, URL arbitraire ou liste d'API générale. Le tableau de bord ne doit pas obtenir de privilège de gestion de fichiers par défaut.

**ACL à deux couches.** Définir la capacité attachée UNIQUEMENT à la fenêtre `main` et limiter les commandes personnalisées via `tauri_build::AppManifest::commands` + permissions générées et vérifiées. Dans Tauri 2, ajouter un identifiant de permission inventé au JSON ne suffit pas ; il faut que cette permission existe réellement au build. Ne jamais accorder `core:webview:allow-internal-toggle-devtools` en Release pour résoudre un bug d'affichage.

**Auth desktop.** Conserver la compatibilité de vérification du hash existant, mais remplacer les cookies/redirects HTTP par un état de session possédé par Rust. La session est invalidée à la fermeture, au logout et après durée d'inactivité spécifiée. Considérer l'isolation du processus local et les secrets existants comme limites du modèle de menace ; pas de promesse de protection absolue contre un malware du même utilisateur.

**CSP.** Déplacer les scripts inline vers des fichiers embarqués, supprimer le besoin de `unsafe-inline` du `script-src`, interdire le chargement distant de code, contrôler `connect-src` et les ressources d'image. `frontendDist` doit être un chemin sur disque, pas une URL.

**Windows lifecycle.** Sous-système Windows sur la crate exécutable en Release (`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` dans `main.rs`), instance unique, arrêt propre des tâches de marché, flush SQLite, erreurs utilisateur non sensibles, logs rotatifs à accès utilisateur. Systray facultatif : l'absence de systray ne constitue pas une faille de sécurité.

**Chaîne de distribution.** WiX/MSI d'abord ; évaluer MSIX séparément selon l'environnement de déploiement. Signer le .exe avant l'assemblage du MSI, signer le MSI et les horodater (RFC 3161). Vérifier chaque signature, l'empreinte et la provenance de build avant publication. La signature ne garantit pas l'absence d'alerte SmartScreen ; sa réputation est distincte.

## 4. Plan des PRs (aucune PR d'implémentation encore créée par RFC-0001)

| PR | Livrable atomique | Dépendances | Test bloquant |
|---|---|---|---|
| 01 | RFC, inventaire HTTP→IPC, modèle de menace et critères de release | aucune | revue/validation des liens et contrats |
| 02 | Services Rust indépendants d'Axum | 01 | tests d'équivalence et invariants moteur/store |
| 03 | Assets frontend embarqués, CSP restrictive, fenêtre Tauri native | 02 | lancement sans réseau local et sans code distant |
| 04 | IPC lecture + événements bornés, DTO versionnés | 03 | contrat UI/IPC, limite de taille/débit |
| 05 | IPC auth + mutation, capacité restrictive et manifeste explicite | 04 | autorisation négative, setup/restart/login |
| 06 | Single-instance, erreurs GUI, logs redacted, fermeture propre | 05 | deux instances, crash, redémarrage |
| 07 | Désactivation du serveur HTTP desktop et suppression scripts de package | 06 | analyse statique + observation réseau |
| 08 | WiX/MSI, migration de données, rollback | 07 | installation propre, upgrade/uninstall |
| 09 | Signature Authenticode, timestamp, attestations/SBOM et fail-closed | 08 | vérification binaire **et** MSI obligatoire |
| 10 | Homologation Windows réelle et politique de release | 09 | matrice tests clean VM et sécurité |

La PR-03 peut construire une UI minimale branchée à un service mock pour éviter de réactiver HTTP ; l'UI complète ne peut être jugée fonctionnelle que lorsque PR-04/05 sont fusionnées. Maintenir les tests existants pendant la migration et supprimer les anciens tests HTTP desktop seulement après leur remplacement.

## 5. Rejets explicites

- Réécrire l'interface en WPF/WinUI 3 sans nécessité démontrée.
- Désactiver Host/Origin ou autoriser `Origin: null` de manière globale pour masquer une régression.
- Distribuer `-ExecutionPolicy Bypass` ou un installeur qui demande l'élévation pour le fonctionnement courant.
- Considérer `TAURI_SIGNING_PRIVATE_KEY` comme la clé Authenticode : ce secret sert au mécanisme de signature de mises à jour Tauri, **distinct** du certificat de signature Windows.
- Déposer un PFX exportable dans une branche, un artefact, un log ou un cache de CI. Favoriser une signature protégée matériellement ou via un fournisseur/KMS ; l'export PFX doit faire l'objet d'un examen de risque strict.
- Promettre `SmartScreen = aucun avertissement` du seul fait de signer ; mesurer plutôt la chaîne de confiance, l'identité éditeur et la réputation dans l'environnement cible.
- Publier le desktop sans certificat/signeur réellement disponible : **publication bloquée**, les builds non signés restent des artefacts de CI internes.

## 6. Documentation de suivi

Inventaire contractuel : [ui-ipc-contract.md](ui-ipc-contract.md).  
Menaces : [threat-model.md](threat-model.md).  
Tests/release : [acceptance.md](acceptance.md).
