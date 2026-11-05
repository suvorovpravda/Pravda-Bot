<div align="center">

# 🛡️ PRAVDA-BOT · SECURITY

### Modèle de sécurité, preuves et limites de production

[![CodeQL](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml)
[![Windows tests](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml)

![IPC](https://img.shields.io/badge/IPC-RUST%20AUTHENTIFI%C3%89-16a34a?style=for-the-badge)
![SQLite](https://img.shields.io/badge/STOCKAGE-SQLITE%20LOCAL-0891b2?style=for-the-badge)
![Trading](https://img.shields.io/badge/ORDRES%20R%C3%89ELS-OFF-16a34a?style=for-the-badge)
![Audit](https://img.shields.io/badge/AUDIT%20EXTERNE-NON%20CERTIFI%C3%89-f59e0b?style=for-the-badge)
![Signed](https://img.shields.io/badge/ENTERPRISE%20SIGN%C3%89-NON%20LIVR%C3%89-dc2626?style=for-the-badge)

**[🏠 README](README.md)** · **[🚦 Statut](#-niveau-de-preuve--5-novembre-2026)** · **[🔐 Contrôles](#-frontières-de-confiance)** · **[🧪 Tests](#-tests-et-rapports-locaux)** · **[🚨 Risques](#-risques-résiduels-et-règles-de-go)** · **[📩 Signalement](#-signaler-une-vulnérabilité)**

</div>

---

> [!CAUTION]
> **Aucune certification de sécurité ou édition Enterprise signée n'est revendiquée.** Les validations automatisées portent sur des commits et des environnements précis. **Trading live OFF** : aucun ordre réel ni retrait ne doit être exécuté par l'interface, les tests, le diagnostic ou les scripts de packaging.

## 🟢 Niveau de preuve — 5 novembre 2026

| Contrôle | État | Preuve / limite |
|:---|:---|:---|
| **Rust, CodeQL, packaging Windows** | 🟢 CI réussie sur `main` `b2fb618b` | [Commit](https://github.com/suvorovpravda/Pravda-Bot/commit/b2fb618b53408b0fd5fd2c4265cdfe0a05ffc898) · [11/11 checks](https://github.com/suvorovpravda/Pravda-Bot/actions?query=branch%3Amain) |
| **MSI / NSIS / premier écran** | 🟢 Tests automatisés sur runner Windows | [Run #41972245070](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/41972245070) |
| **Prérequis WebView2** | 🟢 `offlineInstaller` + `silent=true` dans les deux formats | Configuration prouvée ; **VM initialement sans WebView2/hors réseau toujours non qualifiée** |
| **Révocation de session IPC** | 🟢 Protection intégrée et tests Codex/CI | Ne remplace pas un pentest indépendant |
| **Centre de recette unifié** | 🟠 [PR #139](https://github.com/suvorovpravda/Pravda-Bot/pull/139) | Nouveau binaire **à compiler et valider sur son SHA exact** |
| **Installation modulaire** | 🟠 Non homologuée | Standard / Personnalisé doit retirer réellement permissions et accès |
| **Signature Authenticode** | 🔴 Non attestée | [Issue #90](https://github.com/suvorovpravda/Pravda-Bot/issues/90) |
| **Trading réel** | 🔴 Interdit / non homologué | [Issue #89](https://github.com/suvorovpravda/Pravda-Bot/issues/89) |

La [livraison Windows #106](https://github.com/suvorovpravda/Pravda-Bot/issues/106) a un ticket fermé, **mais un ticket fermé ne constitue pas une preuve de GO**. La cible reste le **30 novembre 2026**, conditionnée à la recette du produit effectivement distribué.

## 🔐 Frontières de confiance

| Surface | Mesure présente | Règle impérative |
|:---|:---|:---|
| **🖥️ WebView2** | HTML/CSS/JS locaux embarqués ; CSP restrictive ; fenêtre principale `main` | Aucun chargement distant privilégié, aucune permission Tauri générique FS/shell |
| **🔏 Tauri IPC** | Commandes nommées, capabilities réduites ; autorisation vérifiée **dans Rust** | Session initiale vérifiée avant **et après** les lectures asynchrones ; rejet après logout/relogin |
| **🔑 Mot de passe** | Minimum 15 caractères ; sel aléatoire, PBKDF2-HMAC-SHA256 (600 000 itérations par défaut) et limitation des tentatives | Ne jamais exposer mot de passe/verifier/token à la WebView ou dans un rapport |
| **🗃️ SQLite** | Base locale, WAL, transactions et lecture bornée | Ne pas supprimer ni « réparer » une base suspecte sans copie cohérente et test de restauration |
| **🪟 Windows Vault** | Secrets gérés en Rust/Win32, pas par JavaScript | Protéger le stockage, la durée de vie, la suppression et les erreurs ; ne pas confondre coffre et protection contre tout malware du même utilisateur |
| **📡 Réseau / opt-in** | Connecteurs publics séparés, lecture Revolut X facultative selon consentement natif | Désactivé par défaut pour les modules optionnels sensibles ; aucune opération broker par un simple booléen d'UI |
| **🧪 Harness** | Tests synthétiques et métriques locales ; pas de mode trading live | Pour attester un mode air-gapped, imposer l'isolement **OS/VM** et contrôler les sorties réseau |
| **🩺 Doctor** | Diagnostic passif local avec sorties autorisées `PASS/WARN/FAIL/SKIP` | Aucun secret, dump SQLite, journal de transaction, réparation destructive ou transmission automatique |
| **📦 Distribution** | MSI / NSIS, SHA-256 et artefacts compagnons comparés par GitHub Actions | Signature **EXE, MSI et NSIS** + horodatage pour toute mention Enterprise |

Le serveur Axum de compatibilité, s'il est lancé explicitement, est **distinct** de l'interface desktop IPC. Il doit rester limité au **loopback**, vérifier Host/Origin/Fetch Metadata et employer une session HTTP authentifiée. Le desktop normal n'ouvre pas de port HTTP pour servir son UI.

### 🔒 Révocation et actions asynchrones

Une réponse appartenant à une session A ne doit jamais être rendue dans une session B, même si une connexion a réussi entre-temps. Le noyau Rust relie le résultat à la session initiale ; l'interface doit également ignorer ses éventuels rejets tardifs avant toute modification d'écran. Ces propriétés ont été durcies dans la [PR #133](https://github.com/suvorovpravda/Pravda-Bot/pull/133) ; l'audit doit les retester sur chaque release candidate.

### 🌐 Proton VPN optionnel

Pravda-Bot ne doit **jamais demander les identifiants Proton VPN**. L'utilisateur configure VPN/kill switch dans l'application officielle. La présence d'un programme ou le clic sur une aide ne prouve **ni tunnel actif ni étanchéité réseau**. Sans attestation fiable, afficher `NON VÉRIFIÉ`. L'activation d'un kill switch susceptible de couper Internet doit rester un choix explicite.

## 🧪 Tests et rapports locaux

### Centre de recette intégré — [PR #139](https://github.com/suvorovpravda/Pravda-Bot/pull/139)

Le nouveau `pravda-production-center.exe` est une **interface de recette en un clic, en cours de qualification** : il regroupe en un seul EXE le desktop, Doctor et Harness construits sur le même commit. Il doit afficher automatiquement un **rapport HTML coloré**, conserver un **TXT expurgé** et exécuter les tests sans outil de développement supplémentaire.

| Étape | Attendu | Limite explicite |
|:---|:---|:---|
| **Doctor embarqué** | Rapport passif local, aucun compte demandé | Un `SKIP` ne vaut jamais `PASS` |
| **Harness embarqué** | Succès/rejet/crash/timeout synthétiques | Les scénarios ne certifient ni stratégie financière ni isolation OS |
| **Desktop temporaire** | Processus et fenêtre visibles, données de test isolées | Ce n'est **pas** un setup/login automatisé, ni une preuve MSI/NSIS |
| **Export** | `Pravda-Production-Report.html` + `Pravda-Production-Report.txt` | **Aucun envoi automatique** à ChatGPT ou à GitHub |

Le partage du rapport est **manuel** après relecture. Si Doctor affiche un problème SQLite, arrêter l'application et préserver une copie cohérente : ne jamais effacer les données pour « réparer » un test.

### Recette qui reste obligatoire avant une distribution complète

1. **VM dédiée Windows x64** avec WebView2 **absent au départ** et réseau désactivé au niveau de l'hyperviseur.
2. Exécution des installateurs **MSI puis NSIS** sur machines/snapshots propres, onboarding réel, redémarrage et préservation des données locales.
3. Tests de refus : archive corrompue, absence de prérequis, profil invalide, source indisponible, session expirée, fenêtre hostile, données SQLite erronées.
4. Validation de l'installation **Standard/Personnalisée**, y compris les permissions et la suppression effective des composants facultatifs.
5. Pour l'édition **Enterprise signée** uniquement : certificat autorisé, chaîne de confiance, Authenticode valide sur **tous les exécutables et installateurs**, horodatage et vérification par un runner indépendant.

> [!WARNING]
> Ne jamais désactiver **SmartScreen, Defender, UAC, TLS ou le pare-feu** pour contourner un échec. Un artefact `unsigned-test` peut être utilisé pour une recette de développement clairement identifiée, **pas** déclaré « Enterprise ».

## 🚨 Risques résiduels et règles de GO

<details>
<summary><b>🟥 Menaces majeures — ouvrir la matrice</b></summary>

| Risque | Impact possible | Exigence de réduction |
|:---|:---|:---|
| Vol/usage abusif d'un secret | Accès aux comptes et aux données privées | Coffre Windows natif, rotation/révocation, journal expurgé, séparation des privilèges |
| Perte de session / interface trompeuse | Réponses anciennes affichées après une nouvelle connexion | Contrôles Rust post-await, génération UI et tests de concurrence |
| Sources ou prix périmés | Décisions trompeuses | Marquer `INDISPONIBLE` / `PÉRIMÉ` ; jamais inventer un cours ou une source |
| Corruption WAL/migration | Perte ou divergence de données | Transactions, sauvegarde cohérente, restauration testée, arrêt sécurisé |
| Distribution substituée | Exécution de binaire malveillant | SHA-256, provenance, Authenticode vérifié et horodatage |
| Flux réseau imprévu | Fuite de données / test offline invalide | Opt-in natif, allowlist, isolation VM/OS et contrôle d'egress |
| Trading non autorisé | Perte financière | Mode live OFF, risk gate fail-closed ; autorisations indépendantes dans #89 |
| Outil de test trompeur | Faux sentiment de conformité | États `PASS/WARN/FAIL/SKIP` et exclusions documentées |

</details>

<details>
<summary><b>🟠 Gouvernance, chaîne d'approvisionnement et incidents</b></summary>

- CI Rust (Windows/Linux), CodeQL et tests de secrets sont des **contrôles automatiques**, pas une évaluation de sécurité indépendante ; étendre l'analyse JS/SCA/SBOM et la provenance de compilation lorsque les preuves sont disponibles.
- Les dépendances, binaires et manifestes doivent être liés au **SHA exact de release**. Une signature GitHub, un SHA-256 ou un statut `Deployed` ne suffisent pas seuls pour attester la sécurité d'un EXE installé.
- Un incident doit préserver les preuves et **révoquer les secrets compromis**, avec analyse sur copie. Ne pas inclure base de données ou identité personnelle dans un ticket public.
- Les connecteurs de recherche, données on-chain et modules avancés doivent rester **isolés et non privilégiés** tant que leur modèle de risque, licences et effets de bord n'ont pas été vérifiés.

</details>

### 📍 Jalons de référence conservés pour le contrôle documentaire

<sub>Les fractions suivantes restent synchronisées avec le [README](README.md) et le [guide](docs/PRAVDA-GUIDE.md). Elles décrivent des **jalons historiques documentaires**, pas le niveau de sécurité actuel.</sub>

| Priorité | Chantier | Jalons de référence | Progression |
|:---|:---|:---|:---|
| **P0** | Séparation Python / Rust | 6/6 | **100 %** |
| **P0** | Sécurité IPC Windows | 2/4 | **50 %** |
| **P0** | Parité Rust métier | 3/8 | **38 %** |
| **P0** | CI et règles GitHub | 3/5 | **60 %** |
| **P1** | Migration SQLite | 3/4 | **75 %** |
| **P1** | Desktop et robustesse | 2/4 | **50 %** |
| **P1** | Distribution Enterprise signée | 0/4 | **0 %** |
| **P2** | Observabilité et charge | 1/4 | **25 %** |
| **P2** | Sources optionnelles et LLM | 0/4 | **0 %** |

## 🔒 Gestion des données et des secrets

- **Ne pas divulguer** mots de passe, `pravda_auth.json`, bases `.db`, WAL/SHM, PFX, tokens, cookies, identifiants d'exchange, journaux bruts ou captures contenant des informations privées.
- Un rapport Doctor ou de recette doit contenir **uniquement des vérifications autorisées** ; ne jamais inclure des valeurs de comptes, chemins personnels ou contenu de base.
- Une sauvegarde SQLite doit être **transactionnellement cohérente**, avant restauration/migration ; l'outil de diagnostic n'effectue **aucune réparation automatique**.
- Les secrets de signature Authenticode sont fournis exclusivement par l'environnement sécurisé de build. Aucun PFX ou mot de passe ne doit être ajouté au dépôt, aux archives distribuées ou aux logs.

## 📩 Signaler une vulnérabilité

> [!IMPORTANT]
> **Ne jamais publier une preuve d'exploitation ou des secrets dans une issue publique.**

1. Ouvrir [GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security) → **Report a vulnerability**, si le signalement privé est activé.
2. Sinon, demander au mainteneur un **canal privé** sans publier les éléments sensibles. Aucune adresse e-mail non vérifiée n'est annoncée.
3. Transmettre en privé le SHA, la version, le contexte Windows, la gravité présumée et une reproduction avec **données de test**.
4. Privilégier une divulgation coordonnée et les tests de non-régression. **Aucun SLA ou programme de récompense** non confirmé n'est revendiqué.

---

### English — security summary

Windows Rust/Tauri local desktop with named authenticated IPC commands, SQLite WAL, passive Doctor diagnostics and deterministic paper-only Harness. **No live orders or withdrawals.** An unsigned CI artifact is **not** a signed Enterprise build; independent external audit and fully air-gapped installation acceptance remain outstanding. Report vulnerabilities **privately** through [GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security).

### Русский — безопасность

Локальное приложение Windows на Rust/Tauri с проверкой IPC в Rust, SQLite WAL и независимыми диагностическими тестами. **Реальная торговля отключена.** Зелёные тесты CI не равны сертификации; подпись Enterprise и установка в изолированной VM требуют отдельных проверок. Уязвимости сообщайте через [приватный раздел GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security).
