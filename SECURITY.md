<div align="center">

# 🛡️ SECURITY · PRAVDA-BOT
### Politique de sécurité · Modèle de menace · Plan de durcissement

![Local](https://img.shields.io/badge/Desktop-IPC%20local-16a34a)
![Access](https://img.shields.io/badge/Acc%C3%A8s-Fen%C3%AAtre%20main%20uniquement-2563eb)
![Auth](https://img.shields.io/badge/Auth-PBKDF2--SHA256-0e7490)
![Audit](https://img.shields.io/badge/Audit%20ind%C3%A9pendant-%C3%80%20compl%C3%A9ter-f59e0b)
![Installer](https://img.shields.io/badge/MSI%20sign%C3%A9-Non%20livr%C3%A9-dc2626)

**[Signalement](#-signaler-une-vulnérabilité)** · **[Frontières](#-frontières-de-confiance)** · **[Contrôles](#-contrôles-effectivement-présents)** · **[Risques](#-risques-résiduels-et-limites)** · **[Plan P0–P2](#-plan-de-sécurisation-priorisé)**

[README](README.md) · [Guide technique unique](docs/PRAVDA-GUIDE.md) · [Avancement P0–P2](docs/PRAVDA-GUIDE.md#-tableau-de-bord-priorisé)

</div>

---

> [!WARNING]
> **Ne pas interpréter « CI verte » ou « desktop sans port HTTP » comme une certification.** Les pourcentages ci-dessous représentent des jalons vérifiables et non un niveau général de sécurité garanti. Les [PR #64](https://github.com/suvorovpravda/Pravda-Bot/pull/64) et [#67](https://github.com/suvorovpravda/Pravda-Bot/pull/67) ont été fusionnées le **2 novembre 2026**. La PR #67 a passé **8/8 checks**, dont une batterie IPC WebView2 sur Windows réel (**22 assertions** avec rapport JSON). Le test utilise **deux sessions natives successives** : il ne prouve ni deux fenêtres simultanées dans le même processus, ni une origine distante hostile. Le propriétaire maintient le projet seul et a **retiré l'exigence d'une review par un second développeur** ; la PR, les checks et la protection sans bypass restent la politique souhaitée. Une capture du paramétrage a été fournie, mais l'API d'administration reste inaccessible à l'intégration (**403**) ; ne pas présenter une vérification API comme acquise. **Aucun MSI Enterprise signé n'est attesté.**

### 📊 État des chantiers et sécurité — priorités

La même échelle que celle du [README](README.md#-avancement-par-priorité) est utilisée ici : **pourcentage = jalons documentés accomplis ÷ jalons prévus**. Les éléments marqués *branche* ne sont **pas fusionnés**. Un test CI ne remplace pas un audit indépendant.

| Priorité | Chantier | Jalons / total | Progression | Preuve ou réserve |
|:--|:--|--:|--:|:--|
| **P0** | Séparation Python / Rust | 3/6 (branche) | **50 %** | Archive seule ; contrôle CI et fusion à valider |
| **P0** | Sécurité IPC Windows | 2/4 | **50 %** | 22 tests Windows et job permanent ; issue #70 ouverte |
| **P0** | Parité Rust métier | 3/8 | **38 %** | Sources et fonctions historiques non entièrement portées |
| **P0** | CI / règles GitHub | 3/5 | **60 %** | Checks actifs ; règles admin non lisibles (HTTP 403) |
| **P1** | Migration SQLite | 3/4 | **75 %** | Tests transactionnels ; reprise/backup Windows à prouver |
| **P1** | Desktop / robustesse | 2/4 | **50 %** | IPC + authentification ; scénarios dégradés à compléter |
| **P1** | Distribution signée | 0/4 | **0 %** | MSI Authenticode non livré |
| **P2** | Observabilité / quotas | 1/4 | **25 %** | Uptime disponible, CPU/RSS/charge à compléter |
| **P2** | Feeds optionnels / LLM | 0/4 | **0 %** | Non démontrés ; pas de succès fictif |

![Sécurité IPC](https://img.shields.io/badge/IPC%20Windows-50%25%20des%20jalons-2563eb)
![Production Rust](https://img.shields.io/badge/Fronti%C3%A8re%20Rust-50%25%20branche-f97316)
![Release](https://img.shields.io/badge/MSI%20sign%C3%A9-0%25-dc2626)

**[Tableau de bord complet, critères de sortie et preuves](docs/PRAVDA-GUIDE.md#-tableau-de-bord-priorisé)**.

## 🇫🇷 Politique — français

### 🎯 Périmètre et garantie réelle

Le présent document couvre **le code Rust actuellement maintenu** :
- `crates/pravda-app/` : frontend Tauri 2/WebView2 embarqué et commandes IPC.
- `rust/pravda-server/` : services Rust partagés, authentification et serveur Axum de **diagnostic séparé**.
- `crates/pravda-core/`, `crates/pravda-store/`, `crates/pravda-engine/` et `crates/pravda-fetchers/` : calculs, SQLite, simulation et réception de marchés.
- Les pipelines sous `.github/workflows/`, le modèle de migration legacy et les scripts de développement Windows.

**Hors garanties :** composants Python archivés (`archive/python-legacy/`), poste déjà compromis par un logiciel malveillant, compte Windows administrateur hostile, réseau d'exchange non fiable, publication de données via un logiciel tiers, paquet binaire distribué en dehors de la chaîne contrôlée.

**Ce que le logiciel ne fait pas :** trading réel, ordres aux exchanges, stockage de clés d'API de trading pour ordonner des transactions, signature Authenticode du paquet actuel. Aucun scénario de sécurité ne doit supposer qu'une simulation papier équivaut à un ordre réel.

**Intégration publique Revolut X :** le module de carnet [#73](https://github.com/suvorovpravda/Pravda-Bot/pull/73) est fusionné, mais sa lecture n'est pas activée dans le runtime desktop. Il ne contient aucune authentification privée ni méthode d'exécution d'ordre. Le coffre Windows proposé en [PR #75](https://github.com/suvorovpravda/Pravda-Bot/pull/75) reste une étape distincte soumise aux tests, sans exposition à l'UI.

### 🔭 Frontières de confiance

```mermaid
flowchart TB
    classDef safe fill:#dcfce7,stroke:#16a34a,color:#14532d,stroke-width:2px
    classDef check fill:#dbeafe,stroke:#2563eb,color:#1e3a8a,stroke-width:2px
    classDef risk fill:#fee2e2,stroke:#dc2626,color:#7f1d1d,stroke-width:2px
    classDef data fill:#fef3c7,stroke:#d97706,color:#78350f

    NET["Internet : Kraken / Binance · données non fiables"]:::risk --> VALID["Validation prix / temps / provenance"]:::check
    subgraph WINDOWS["Compte utilisateur Windows · pravda-app.exe"]
        UI["WebView2 · HTML/JS/CSS embarqués"]:::check --> ACL{"Tauri ACL + fenêtre main"}:::check
        ACL -->|refus| DENY["FORBIDDEN_WINDOW / ACL deny"]:::risk
        ACL -->|autorisé| CMD["Commande IPC typée Rust"]:::safe
        CMD --> LOGIN{"Bootstrap auth ou session valide ?"}:::check
        LOGIN -->|non| DENY2["UNAUTHORIZED"]:::risk
        LOGIN -->|oui| ENGINE["DesktopRuntime + calculs"]:::safe
        ENGINE --> DB[("SQLite + WAL · historique")]:::data
        LOGIN --> AUTH["PBKDF2 salé · jeton en mémoire Rust"]:::safe
        AUTH --> FILE["pravda_auth.json"]:::data
    end
    VALID --> ENGINE
    BROWSER["Navigateur diagnostic"]:::check -->|"HTTP/WS loopback uniquement"| AXUM["pravda-server · session cookie"]:::check
```

| Frontière | Confiance accordée | Rejet exigé |
|:--|:--|:--|
| Internet → fetchers | **Aucune** sur les cotations reçues | Valeurs NaN/∞, négatives, périmées et écarts incohérents |
| WebView2 → commandes Rust | Fenêtre `main` + capacité nommée + session | Fenêtre non autorisée, non connecté, entrées malformées |
| Auth Rust → stockage | Chemins explicites, identité Windows, vérificateur PBKDF2 | Enregistrement auth corrompu, chemin invalide |
| Desktop → SQLite | Requêtes paramétrées et bornées | SQL arbitraire depuis JS et export vers chemin fourni par UI |
| Navigateur → Axum | Loopback, `Host` et `Origin` cohérents, cookie valide | Origine étrangère, rebinding, accès non authentifié |
| CI → publication | Commit, checks et artefacts vérifiés | MSI non signé, secrets dans artefact, checks incomplets |

### ✅ Contrôles effectivement présents

<details open>
<summary><strong>1. Authentification et sessions locales</strong></summary>

- Mot de passe local **≥ 15 caractères** et **≤ 1 024 octets**.
- Vérificateur **PBKDF2-HMAC-SHA256** avec sel aléatoire et **600 000 itérations**, stocké dans `pravda_auth.json` ; le fichier ne contient pas le mot de passe en clair.
- Création initiale refusée si un vérificateur existe déjà. Un fichier de vérification invalide empêche le démarrage au lieu de réinitialiser silencieusement l'authentification.
- **Verrouillage dès le 5e échec**, avec délai progressif (base 30 s, plafond 15 min, selon l'implémentation). Validité nominale de session : **12 heures** ; révocation lors du logout.
- Dans Tauri, **le jeton de session reste dans Rust**, via `IpcAuth` : il n'est pas renvoyé au JavaScript. Les opérations PBKDF2 s'exécutent hors thread WebView.
- Dans Axum, les sessions navigateur utilisent un **cookie HttpOnly / SameSite=Strict**, sans transmettre le secret par les snapshots métier.

**Limite :** ce mécanisme ne résiste pas à un malware ayant accès au compte utilisateur, à la mémoire de processus ou aux fichiers autorisés.

</details>

<details open>
<summary><strong>2. IPC Tauri minimal et ressources locales</strong></summary>

- UI depuis `WebviewUrl::App("index.html")` et `build.frontendDist="frontend"` ; pas de chargement initial d'un site distant.
- `crates/pravda-app/capabilities/main.json` donne des permissions **individuelles** aux commandes de la seule fenêtre `main` ; aucune permission `shell`, `fs` globale, aucune origine distante déclarée.
- **12 commandes nommées**, enregistrées dans le manifeste Tauri ; 5 liées à auth/session et 7 lectures :
  - Bootstrap/session : `ipc_auth_status`, `ipc_auth_setup`, `ipc_auth_login`, `ipc_auth_logout`, `ipc_session_check`.
  - Métier : `ipc_market_snapshot`, `ipc_history_recent`, `ipc_history_query`, `ipc_history_export_csv`, `ipc_paper_alerts_snapshot`, `ipc_runtime_metrics`, `ipc_fx_status`.
- Les commandes de bootstrap valident la **fenêtre `main`** ; les opérations protégées appellent **`require_authenticated()` côté Rust**, en plus de l'ACL Tauri.
- Content Security Policy (CSP) desktop : `script-src 'self'` et `style-src 'self'`, `object-src 'none'`, `frame-src 'none'`, `form-action 'none'`, `base-uri 'none'`. `connect-src` autorise uniquement les schémas internes du pont IPC (`ipc:` et `http://ipc.localhost`). **Cette autorisation ne signifie pas qu'Axum écoute sur ce port.**
- Commandes d'historique avec filtres typés et réponses bornées. Le CSV provient de lignes validées et non d'un chemin de fichier arbitraire.
- Le desktop **ne démarre pas de serveur HTTP ni de WebSocket local pour l'UI**. Démarrage Windows et absence de socket `Listen` sur son PID testés manuellement ; le smoke CI le contrôle aussi.

**Limite :** la preuve d'un **refus IPC dynamique depuis une seconde fenêtre ou origine hostile** n'a pas encore été consignée. Une protection écrite dans le code n'est pas un test d'intrusion.

</details>

<details>
<summary><strong>3. Serveur Axum de diagnostic (transport différent)</strong></summary>

- Écoute **127.0.0.1** uniquement ; `DASHBOARD_PORT` validé et réservé ; le port n'est pas utilisé comme transport UI du desktop.
- Vérification stricte de `Host`, `Origin`, du port et des métadonnées navigateur `Sec-Fetch-*` ; rejets d'origines étrangères, de doublons d'en-têtes et d'hôtes arbitraires.
- Exception **bornée** de navigation WebView2 historique pour certains documents publics ; **pas** de permission générale sur les API ou WebSocket.
- En-têtes défensifs `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer` et `Cross-Origin-Resource-Policy: same-origin`.
- Accès au dashboard métier, API et WebSocket conditionné par la session serveur. Les clients CLI locaux sans `Origin` restent possibles et ne constituent pas une barrière contre un processus local malveillant.

**Ne jamais exposer ce serveur par un proxy public, un tunnel, un transfert NAT ou une redirection de port.** « Loopback » ne signifie pas « isolé de tous les processus du poste ».

</details>

<details>
<summary><strong>4. Intégrité des données et migration SQLite</strong></summary>

- Historique SQLite avec journal **WAL**, accès SQL paramétré et filtres symbol/exchange/période sous formes d'énumérations.
- Réponses d'historique **limitées à 10 000 événements**, indicateur de troncature ; pas d'injection de SQL utilisateur depuis le frontend.
- Import legacy Python : source ouverte **en lecture seule**, transaction d'import cible, table de provenance des lignes importées, rollback intégral si une ligne invalide bloque le lot.
- Un chemin legacy explicite absent/invalide produit une erreur, **pas** une création silencieuse d'une base de remplacement.
- **Attention :** le chemin canonique de la base source fait partie de l'identité de réimport ; la déplacer peut faire considérer ses lignes comme une nouvelle source.
- Répertoires Windows utilisateur : `%LOCALAPPDATA%\Pravda-Bot` pour SQLite et auth, sauf chemins d'environnement explicites.

**Sauvegarde :** arrêter les processus, puis effectuer une sauvegarde cohérente SQLite (API de backup ou copie valide de la base et de son WAL) **et** du fichier auth. Une copie à chaud du seul `.db` ne garantit pas l'intégrité.

</details>

<details>
<summary><strong>5. CI, dépendances et comportement de marché</strong></summary>

- CI Rust Linux/Windows, MSRV **1.95.0**, `fmt`, `clippy`, tests, compilation Release, contrats JS/IPC statiques, build/smoke Tauri Windows, scénario serveur navigateur et CodeQL.
- Dependabot surveille des dépendances Cargo, GitHub Actions, archive Python et une configuration npm ; les PR de dépendances doivent rester soumises aux validations normales.
- Flux publics : validation des cours **finis et strictement positifs**, contraintes de fraîcheur/désynchronisation et écarts avant la simulation papier.
- Valeurs indisponibles : renvoi de `null` ou état `unavailable` lorsqu'une source/métrique (FX, CPU/RSS, etc.) n'est pas effectivement implémentée.
- **Aucun ordre en argent réel** ; le moteur ne modélise pas des fills garantis ni tous les coûts d'exécution.

**Limite :** un scan CodeQL vert ou une compilation reproductible ne constitue pas, à eux seuls, un audit de sécurité indépendant.

</details>

### 🔎 Matrice de menaces et état des protections

| Menace | Actif | Défense intégrée | Vérification / risque restant |
|:--|:--|:--|:--|
| XSS / script externe | WebView2 | JS/CSS locaux, CSP `'self'` | Tests d'injection et revue du frontend à compléter |
| Appel IPC non autorisé | Données métier | ACL `main` + autorisation Rust | **Tests Windows natifs validés PR #67** : 8 refus `UNAUTHORIZED` sans session, refus après logout, refus ACL `security-probe` ; **origine distante et fenêtres simultanées non testées** |
| Vol local de session | Session | Jeton côté Rust ; révocation logout | Malware même utilisateur hors garantie |
| Brute force mot de passe | Auth | PBKDF2 + verrouillage progressif | Tester sur redémarrage/attaques répétées |
| SQL injection / export arbitraire | SQLite | Requêtes paramétrées, enums, CSV borné | Revue du format et test CSV malveillant |
| Corruption de migration | Historique | Lecture seule source, transaction et provenance | Restaurations et crash tests à compléter |
| DNS rebinding / CSRF HTTP | Serveur diagnostic | Host/Origin/Fetch Metadata et cookies | Processus local hostile non exclu |
| Déni de service local | Disque / CPU / flux | Requêtes historiques bornées | Limites globales connexions, débit et disque **à faire** |
| Dépendance compromise | Build | CI, CodeQL, Dependabot | Audit/attestation SBOM et provenance à faire |
| Exécutable remplacé | Poste Windows | Aucune signature Enterprise attestée | **P0 release :** signature éditeur, horodatage et contrôle chaîne |
| Contournement de gouvernance | `main` | Protection activée ; capture propriétaire : « Do not allow bypassing » coché | **L'enregistrement effectif n'est pas lisible via l'intégration (HTTP 403)** ; gouvernance solo sans approbateur externe |

### ⚠️ Risques résiduels et limites

> [!CAUTION]
> **Ce qui n'est PAS garanti :** protection complète contre un malware local, absence de vulnérabilité, confidentialité face à l'administrateur du poste, résultats financiers réels, qualité intrinsèque des sources de marché ou intégrité d'un exécutable non signé téléchargé ailleurs.

1. **IPC :** [preuves natives WebView2 de la PR #67](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/41469749796/job/125359902577) : refus Rust avant/après session, refus ACL d'une WebView `security-probe` distincte et refus `fs`/`shell`. **Restent hors preuve** : deux fenêtres concurrentes partageant un processus et une session authentifiée, navigation depuis origine distante et tests d'injection XSS.
2. **Distribution :** `bundle.active=false` et **aucun MSI signé avec Authenticode/RFC 3161** n'est attesté ; les scripts de lancement de développement ne constituent pas une chaîne de release.
3. **Charge / déni de service :** pas de garantie de quota global sur toutes les connexions, sessions, flux, requêtes et la taille de la base. Les limites d'une **réponse** ne sont pas une limite de volume du disque.
4. **Disponibilité :** FX, énergie, actualités, AIS, notifications et mesures CPU/RSS n'ont pas tous un backend réel complet ; afficher l'absence de données plutôt que créer des valeurs.
5. **Legacy :** une ancienne base Python, ses scripts et ses dépendances restent des matériaux de référence, **pas** des composants héritant automatiquement de la sécurité Rust.
6. **Gouvernance d'un dépôt solo :** le propriétaire a décidé de **ne plus exiger de reviewer externe**. Cette décision n'est pas une revue indépendante rétroactive de #64/#67. La capture des paramètres montrait la protection « no bypass » cochée ; la configuration effectivement sauvegardée n'est pas directement vérifiable par cette intégration (**403**). Ne jamais lever les checks requis pour forcer une fusion.

### 🗺️ Plan de sécurisation priorisé

```mermaid
flowchart LR
    classDef p0 fill:#fee2e2,stroke:#dc2626,color:#7f1d1d,stroke-width:2px
    classDef p1 fill:#fef3c7,stroke:#d97706,color:#78350f,stroke-width:2px
    classDef p2 fill:#dbeafe,stroke:#2563eb,color:#1e3a8a,stroke-width:2px
    A["P0 · Prouver les protections GitHub et CI post-merge"]:::p0
    B["PR #67 · IPC Windows E2E vérifié · 22 assertions"]:::p0
    C["P1 · Migration, CSV, limites de charge et logs"]:::p1
    D["P2 · Supply chain, SBOM et MSI signé"]:::p2
    A --> B --> C --> D
```

| Étape | Action attendue | Preuve de clôture |
|:--|:--|:--|
| **P0 — immédiat** | Vérifier les résultats des jobs **sur le SHA `main` fusionné**, pas seulement sur la tête de PR | Liens CI, SHA et conclusion de chaque job |
| **P0 — gouvernance** | Solo : PR et checks requis, branche à jour, commentaires résolus et « Do not allow bypassing » ; **aucun reviewer indépendant obligatoire** | Capture enregistrée / validation des paramètres effectifs par l'administrateur |
| **P0 — IPC** | **Acquis dans #67 :** refus sans session, après logout et fenêtre `security-probe` isolée. **Restant :** origine distante et seconde fenêtre simultanée dans un même processus | Logs de la [CI #67](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/41469749796/job/125359902577) ; futur scénario natif complémentaire |
| **P1 — stockage** | Import SQLite legacy avec doublons, fichier corrompu, interruption, rollback/reprise ; vérifier export CSV et refus de formules | Jeux de données fictives + tests reproductibles |
| **P1 — robustesse** | Tester erreurs/charge, reconnections exchange, quotas disque, masquage des logs | Tests automatisés + résultat de saturation |
| **P2 — dépendances** | Cartographier dépendances et produire SBOM, audit et provenance artefacts | SBOM + rapport d'audit associé à un SHA |
| **P2 — livraison** | Signer **EXE et MSI** Authenticode, horodater RFC 3161, vérifier sur Windows propre, sécuriser rollback | Signatures `signtool` et tests install/upgrade/uninstall |

**Règle de publication :** tant qu'un certificat de signature approuvé, les tests de Windows réel, l'intégrité de la chaîne de build et une revue de sécurité ne sont pas démontrés, **aucune promesse de release Enterprise prête à déployer**. Les jalons P0/P1/P2 ne sont pas des échéances calendaires.

### 🧪 Procédures de contrôle reproductibles

**Vérifier le contrat source et les tests automatiques :**

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo test --locked --manifest-path .\crates\pravda-app\Cargo.toml
node tools/test-desktop-ipc.mjs
node tools/test-dashboard.mjs
```

**Vérifier le réseau du desktop** (application ouverte, depuis PowerShell) :

```powershell
$app = @(Get-CimInstance Win32_Process |
    Where-Object { $_.Name -eq "pravda-app.exe" })
if ($app.Count -eq 0) {
    throw "Le desktop Tauri n'est pas lancé."
}
$listeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
    Where-Object { $app.ProcessId -contains $_.OwningProcess })
if ($listeners.Count -ne 0) {
    $listeners | Select-Object LocalAddress, LocalPort, OwningProcess
    throw "Écoute TCP détectée sur le processus desktop."
}
Write-Host "OK : aucun listener TCP pour pravda-app.exe."
```

**Ce test vérifie uniquement les sockets en écoute de ce processus**, pas les connexions sortantes ou les autres programmes Windows.

**Scénarios de sécurité à ne pas sauter :** tentative d'invocation avant setup/auth, identifiants faux, logout suivi d'accès IPC, fenêtre secondaire sans capability, tentative de chargement externe, expiration, chemin SQLite invalide, migration interrompue, import répété, CSV avec données hostiles. Utiliser **uniquement des données de test**.

### 🔒 Gestion des données et des secrets

- **Ne jamais publier** mots de passe, `pravda_auth.json`, bases SQLite utilisateur, fichiers WAL, clés/tokens, environnements, topics de notification privés, chemins absolus personnels ou journaux bruts.
- Ne pas stocker un mot de passe dans une capture ou un script partagé ; les variables d'environnement restent observables par des logiciels disposant des droits nécessaires.
- Ne pas versionner les bases réelles dans Git. `.gitignore` n'efface **pas** des secrets déjà commités : en cas d'exposition, **révoquer/renouveler** les identifiants concernés et examiner l'historique.
- Ne pas publier de fichiers `.pfx` de signature ni leur mot de passe dans le dépôt ou les artefacts CI.
- Faire des sauvegardes **cohérentes** et tester une restauration avec des données fictives avant toute migration réelle.
- Vérifier l'URL/éditeur et le SHA d'un artefact ; un SHA-256 hébergé au même endroit qu'un exécutable compromis n'est pas une preuve suffisante d'authenticité.

### 📩 Signaler une vulnérabilité

> [!IMPORTANT]
> **Ne publiez jamais un exploit, un mot de passe, une base personnelle ou une trace contenant des données sensibles dans une issue publique.**

1. Ouvrir la [page Security du dépôt](https://github.com/suvorovpravda/Pravda-Bot/security) et chercher **« Report a vulnerability »** si la fonctionnalité **Private vulnerability reporting** est activée.
2. Si elle n'apparaît pas, **demander au mainteneur un canal privé** sans communiquer la preuve d'exploitation dans une issue publique. Aucun email de sécurité non vérifié n'est annoncé ici.
3. Inclure, **dans le canal privé**, la version/commit, le système, la catégorie d'impact, les prérequis, des étapes minimales et des données de test non sensibles.
4. Préférer la divulgation coordonnée : laisser le temps de confirmer, corriger, tester puis publier une note de sécurité. **Aucun délai de réponse ou SLA non vérifié n'est promis.**

**Attentes du mainteneur :** accuser réception si possible, reproduire sur un environnement isolé, évaluer la gravité, corriger dans une branche revue, ajouter un test de non-régression et communiquer sur les versions réellement touchées. Le projet n'affirme ni programme de bug bounty ni garantie d'indemnisation.

---

## 🇬🇧 Security policy — English

**Scope.** Maintained Rust/Tauri 2 Windows desktop, separate Axum loopback diagnostic server, shared authentication, market validation, SQLite storage and CI. Archived Python is **not** part of the active desktop runtime.

**Desktop boundary.** Bundled WebView2 local assets call named Tauri IPC commands; permissions are scoped to the `main` window and the Rust command handlers enforce session checks. The session token stays in Rust, never in JS. CSP restricts scripts/styles to local resources; no generic filesystem or shell permissions are granted. The desktop does **not** open an HTTP listener for its UI.

**Authentication.** Minimum 15 characters, salted PBKDF2-HMAC-SHA256 with 600,000 iterations. Failed attempts trigger progressive lockout; logout revokes the active session. The separate Axum browser mode binds to loopback, validates Host/Origin/Fetch Metadata, and uses authenticated HttpOnly/SameSite=Strict session cookies.

**Persistence.** SQLite WAL, parameterized and bounded history reads, read-only source for transactional Python legacy migration, and per-user Windows AppData defaults. Back up the database **including its WAL consistency** and the password verifier before migrating.

**Verified and outstanding.** Merged [PR #67](https://github.com/suvorovpravda/Pravda-Bot/pull/67) passed eight CI checks, including **22 real Windows WebView2 IPC assertions** (unauthenticated and logged-out denials, unprivileged probe, denied file/shell commands). These are **two successive native sessions**, not concurrent windows in one authenticated process; remote-origin behavior also remains untested. The solo maintainer removed the requirement for a second human reviewer. Branch no-bypass was shown checked in a maintainer screenshot, but the integration cannot query saved settings (**403**). A signed MSI/Authenticode, SBOM, load limits and some feeds are pending. CI is **not** a security certification.

**Responsible disclosure.** Use [GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security) → **Report a vulnerability** if private reporting is enabled. Otherwise ask the maintainer for a **private contact route without posting exploit details publicly**. Share reproducible non-sensitive evidence privately. No guaranteed response SLA is published.

---

## 🇷🇺 Политика безопасности — русский

**Область действия:** текущий код Rust/Tauri 2, отдельный диагностический сервер Axum, SQLite, аутентификация и CI. Архив Python не является рабочим процессом desktop.

**Desktop:** локальные HTML/CSS/JS и IPC Tauri; разрешения только для окна `main`, проверка сессии в Rust. Токен сессии не передаётся JavaScript. UI **не открывает локальный HTTP-порт**; сервер диагностики Axum работает отдельно и использует loopback, проверку Host/Origin и cookie-сессию.

**Пароль:** минимум 15 символов, случайная соль, PBKDF2-HMAC-SHA256 (600 000 итераций), задержки после ошибок, отзыв сессии при выходе. Данные сохраняются в SQLite/WAL и файле проверки пароля в пользовательском каталоге Windows. При миграции нужны согласованные резервные копии.

**Проверено и ограничения:** [PR #67](https://github.com/suvorovpravda/Pravda-Bot/pull/67) объединён, **8 проверок CI и 22 IPC-теста на настоящем Windows/WebView2 прошли успешно**: отказы без сессии, после выхода, для окна `security-probe` и запрещённых команд shell/fs. Тесты выполнены в двух **последовательных процессах**; одновременные окна одного процесса и удалённое происхождение не проверены. Единственный сопровождающий отменил обязательное одобрение второго разработчика; настройка no-bypass показана на скриншоте, но API интеграции возвращает 403. Подписанный MSI, SBOM и ограничения нагрузки остаются задачами.

**Уязвимости:** используйте [закрытую отправку через GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security), если она включена, либо запросите у сопровождающего приватный способ связи. Не публикуйте пароли, данные пользователей или подробности эксплуатации в открытых issues.
