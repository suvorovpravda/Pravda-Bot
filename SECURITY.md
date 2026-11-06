<div align="center">

# PRAVDA / SECURITY

### La confiance se démontre.

**Architecture à permissions minimales · Données locales · Publication vérifiable**

[État](#état-de-sécurité) &nbsp;·&nbsp; [Frontières](#frontières-de-confiance) &nbsp;·&nbsp; [Menaces](#risques-résiduels) &nbsp;·&nbsp; [Publication](#sécurité-de-la-publication) &nbsp;·&nbsp; [Signaler](#signaler-une-vulnérabilité)

[README](README.md) &nbsp; / &nbsp; [Reviewing](docs/REVIEWING.md) &nbsp; / &nbsp; [Registre P0](https://github.com/suvorovpravda/Pravda-Bot/issues/160)

</div>

---

## État de sécurité

**06 novembre 2026 · NO-GO pour une release de production.**

Aucune certification indépendante ou garantie d'absence de vulnérabilité n'est revendiquée. Un contrôle CI réussi prouve uniquement le scénario testé **sur le SHA mentionné**. Il ne remplace ni une recette sur l'installateur réel ni un contrôle indépendant de la chaîne de confiance.

**Incident d'intégration avéré :** la [CI Windows de la PR #157](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42290188079) a constaté une initialisation partielle du tableau de bord sur un EXE **installé**. La publication reste bloquée. L'intégration des corrections se fait dans la [PR #159](https://github.com/suvorovpravda/Pravda-Bot/pull/159).

| Domaine | État | Preuve ou manque |
| :--- | :--- | :--- |
| Isolation Tauri / IPC et authentification Rust | Contrôles implémentés | Revérifier sessions et permissions sur chaque modification |
| Stockage SQLite et vérificateur d'authentification | Doctor local : contrôles passifs | La disponibilité du fichier ne valide pas le parcours graphique |
| Build Rust / CodeQL / tests WebView2 | Couverture CI disponible | Les checks doivent passer sur **le même commit** que l'installeur |
| Installer MSI / NSIS | **NO-GO** | Onboarding GUI installé à valider, un échec confirmé |
| VM sans WebView2 et sans Internet | **Non attestée** | Isolement hyperviseur et deux snapshots propres requis |
| Installation Standard / Personnalisée | **Non homologuée** | Les modules masqués doivent aussi être refusés côté Rust |
| Dépendances desktop RustSec | **Risque ouvert** | Vulnérabilité sur le chemin historique h2 0.3, à vérifier et éliminer |
| Authenticode / signature Enterprise | **Non attestée** | Certificat, identité, horodatage et vérification indépendante |
| Updater Tauri signé | **Désactivé** | Chaîne cryptographique et scénarios de restauration non homologués |
| Ordres réels / retraits | **Interdits dans l'édition test** | Consentement et homologation financière indépendants |

Toutes les exigences et preuves à obtenir figurent dans l'[Issue #160 — Security / Déploiement](https://github.com/suvorovpravda/Pravda-Bot/issues/160). La configuration d'installation Tauri utilise `offlineInstaller` pour le prérequis WebView2, **mais l'essai réellement hors réseau sur VM vierge reste NO-GO**. Ne pas clore une vulnérabilité au seul motif que son suivi a été déplacé.

## Frontières de confiance

| Frontière | Garantie requise | Comportement interdit |
| :--- | :--- | :--- |
| **WebView → Rust** | Allowlist de commandes, session authentifiée, validation stricte des entrées | Accès à une commande protégée par manipulation du DOM |
| **Session → résultat différé** | Lier chaque réponse à sa session ; révoquer les réponses périmées | Afficher ou réutiliser les données d'une ancienne session |
| **Coffre → application** | Secrets conservés dans le code natif ; permissions minimales | Mot de passe, clé privée, cookie ou jeton dans IPC, JS ou logs |
| **Flux publics → données** | Source autorisée, horodatage, état de fraîcheur et refus sécurisé | Prix fictif présenté comme réel ou statut live déduit d'un socket non vérifié |
| **SQLite → diagnostic** | Lecture seule, WAL cohérent, sauvegarde et restauration contrôlées | Suppression/réinitialisation automatique des données utilisateur |
| **CI → utilisateur** | SHA exact, artefacts et somme de contrôle, signature vérifiée si revendiquée | Publier un candidat non signé comme édition certifiée |
| **Trading simulation → live** | Séparation technique et autorisation explicite indépendante | Transformer un contrôle de simulation en ordre réel |

Les permissions de fenêtre Tauri et la CSP doivent demeurer restrictives. Aucun script distant ni serveur HTTP de contrôle local n'est requis pour le fonctionnement de base du bureau.

### Gestion des identifiants et des données

Le bureau n'expose pas de clés d'exchange, de secrets du coffre Windows ou de mot de passe au code WebView. Les rapports destinés au support sont **expurgés** et consultables par l'utilisateur avant tout partage.

**Ne jamais transmettre** : fichiers d'authentification, clés API ou de signature, fichiers PFX, bases SQLite, fichiers WAL/SHM, identifiants de comptes, journaux bruts, jetons de session ou historiques privés.

Les diagnostics doivent être non destructifs et explicitement marquer **PASS, WARN, FAIL ou SKIP**. Les scénarios hors réseau ne doivent pas lancer de connexion privée ou financière. Un SKIP n'est ni un PASS ni un échec dissimulé.

## Risques résiduels

| Priorité | Risque | Barrière |
| :--- | :--- | :--- |
| **P0** | Dashboard incomplet après création d'un mot de passe | Test du binaire **installé**, initialisation JS complète, première connexion et reconnexion |
| **P0** | Windows lance une console liée au bureau | Vérifier le sous-système PE GUI=2 sur l'exécutable de la release |
| **P0** | Flux publics muets, bloqués ou périmés | Délais d'attente, backoff, états distincts ; preuve de ticks réels horodatés |
| **P0** | Dépendance vulnérable dans le lock desktop | Analyse Cargo et RustSec distincte du workspace ; correction sans allowlist cosmétique |
| **P0** | Paquet remplacé ou altéré | Manifeste SHA-256, provenance, Authenticode valide pour publication signée |
| **P0** | Mise à jour cassant SQLite ou abaissant la version | Updater OFF ; E2E refus de downgrade, corruption, interruption et rollback |
| **P0** | Installation airgap prétendument validée | VM jetable réellement isolée au niveau hyperviseur, sans WebView2 préalable |
| **P1** | Compromission du compte Windows local | Aucune promesse de protection du coffre contre un malware exécuté sous le même utilisateur |
| **P1** | Choix d'installation purement décoratif | Capacités de modules désactivées et contrôlées dans Rust, pas seulement cachées |
| **P1** | Trading ou retrait non autorisé | Ordres réels OFF ; circuit de décision indépendant dans l'Issue #89 |

### Dépendances et RustSec

L'audit de l'arbre **desktop** a identifié le chemin historique **reqwest 0.11.27 → hyper 0.14.32 → h2 0.3.27** et l'avis **RUSTSEC-2026-0258** ; il a également signalé des dépendances non maintenues. Les analyses non bloquantes peuvent afficher un workflow global vert malgré une commande d'audit qui retourne un code d'échec. Ce problème est suivi dans l'[Issue #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160), rubrique RustSec.

Avant toute publication signée : inspecter les dépendances inverses, régénérer Cargo.lock **avec Cargo**, traiter les avis et rendre l'audit desktop bloquant. Ne jamais masquer un avis vulnérable sans justification de sécurité et approbation explicite.

## Sécurité de la publication

Le pipeline ne vaut **GO** que si toutes les vérifications suivantes se rapportent au même SHA et à l'artefact effectivement distribué.

- [ ] Rust fmt, clippy, tests Rust Windows/Linux, MSRV, CodeQL et tests de sécurité IPC.
- [ ] Audit RustSec du lock desktop et de l'espace de travail, sans vulnérabilité de sécurité non traitée.
- [ ] Binaire Windows GUI sans console liée au terminal.
- [ ] Installation MSI et NSIS, création du mot de passe, tableau de bord entièrement initialisé, redémarrage, login/logout/re-login, désinstallation et conservation SQLite.
- [ ] Test center isolé, une seule instance, arrêt contrôlé, aucun secret dans les rapports.
- [ ] Provenance, empreintes, signatures/horodatages indépendamment vérifiés **si la distribution est annoncée signée**.
- [ ] Test d'installation sur VM sans WebView2 ni réseau pour toute promesse d'installation totalement hors ligne.
- [ ] Déploiement progressif et restauration sans perte ; mécanisme de refus et rollback du signed updater testé.
- [ ] Consentement réseau public distinct ; aucune clé privée ni capacité d'ordre réel activée.

**Édition unsigned-test :** uniquement identifiée comme telle, non assimilée à une Enterprise. Aucun contournement de Windows Defender, SmartScreen, UAC ou du pare-feu. Une réputation SmartScreen ne s'obtient pas par la seule signature cryptographique.

**Updater :** la [PR #153](https://github.com/suvorovpravda/Pravda-Bot/pull/153) a intégré la vérification de signatures de **candidats** ; elle ne constitue pas un updater automatique opérationnel. Le plugin dormant de la [PR #154](https://github.com/suvorovpravda/Pravda-Bot/pull/154) n'est pas publié comme mécanisme de mise à jour actif. Clé publique épinglée, manifestes authentifiés, contrôles de version, connexion HTTPS autorisée, refus de downgrade, rollback et consentement restent des barrières obligatoires.

### Processus de reviewing

Toute PR modifiant Rust, Tauri, l'authentification, le stockage, les flux, les installateurs ou l'updater suit la grille de [docs/REVIEWING.md](docs/REVIEWING.md). Une revue humaine ou automatisée sans preuve de tests sur le SHA final ne constitue pas une approbation de production. Les commentaires critiques ne doivent pas être résolus avant correctif et vérification.

## Signaler une vulnérabilité

Utiliser de préférence le [centre GitHub Security du dépôt](https://github.com/suvorovpravda/Pravda-Bot/security) et son canal privé de signalement lorsqu'il est activé. **Ne pas publier de preuve d'exploitation contenant un secret dans une Issue publique.**

Un rapport utile précise la version ou le SHA affecté, le comportement reproductible, l'impact et les étapes minimales non destructives, avec une capture expurgée si nécessaire. Aucun accès aux clés API, à la base d'un utilisateur ni à un compte financier n'est nécessaire pour rédiger un signalement.

---

**EN** — Production release: **NO-GO**. GitHub CI success is scoped to specific checks on a specific commit. Native Windows onboarding, RustSec, provenance and signed updater controls must be verified independently. Live orders are disabled.

**RU** — **Продакшен-релиз не разрешён** до проверки установленного приложения, зависимостей RustSec и цепочки подписи. Настоящие ордера отключены. Не публикуйте ключи и токены в открытых Issue.
