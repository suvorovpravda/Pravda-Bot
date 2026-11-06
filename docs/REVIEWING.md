<div align="center">

# PRAVDA / REVIEWING

### Preuves avant promesses.

**Un changement · Un commit · Une chaîne de tests · Une décision**

[README](../README.md) &nbsp; / &nbsp; [Security](../SECURITY.md) &nbsp; / &nbsp; [Registre des blocages #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160)

</div>

---

## Principe de revue

L'objectif n'est pas d'accumuler les Pull Requests ou les badges verts. Le reviewer doit pouvoir répondre clairement à trois questions :

**Qu'est-ce qui a changé ? Qu'est-ce qui a réellement été testé ? Qu'est-ce qui reste non démontré ?**

- Une affirmation de sécurité se fonde sur un **contrôle reproduisible**, pas sur l'existence d'une fonction.
- Le résultat d'une branche différente ne valide pas l'artefact candidat.
- Le code, l'installateur et les rapports doivent référencer le **SHA exact** et, si nécessaire, le SHA-256 du binaire.
- La simulation n'est pas une preuve de données de marché live, et l'absence de données n'est jamais un prix nul inventé.
- La clôture d'une PR ne transforme pas un bug non corrigé en bug résolu. Déplacer un problème dans une autre Issue conserve son **NO-GO**.

## Niveaux de décision

| Décision | Signification |
| :--- | :--- |
| **READY FOR REVIEW** | Modification implémentée ; preuve CI et documentation soumises, mais non approuvées |
| **CHANGES REQUESTED** | Blocage fonctionnel, sécurité, donnée ou preuve manquante |
| **APPROVED FOR MERGE** | Revue du diff final et contrôles requis réussis au même SHA |
| **APPROVED FOR TEST** | Build installé et testé pour les scénarios déclarés, avec limites explicites |
| **GO PRODUCTION** | Critères de sécurité et de déploiement satisfaits, signatures et rollback qualifiés |
| **NO-GO** | Au moins une condition de publication obligatoire est manquante ou en échec |

Une PR en **draft** n'est pas une livraison. La validation du code et la validation d'une installation constituent **deux décisions distinctes**.

## Grille de revue obligatoire

### 01 — Architecture et permissions

- [ ] Modification compatible avec Rust/Tauri 2 et la séparation native IPC / WebView.
- [ ] Aucun serveur local, runtime Python/Node de production ou permission supplémentaire sans besoin justifié.
- [ ] Session Rust vérifiée sur chaque IPC sensible ; résultats asynchrones liés à leur session.
- [ ] Aucune clé, aucun token, mot de passe, PFX ou information privée dans le diff, les logs ou le rapport.
- [ ] Absence d'ordre réel, retrait ou activation implicite de trading live.

### 02 — Frontend et expérience Windows

- [ ] Test du **vrai exécutable installé** depuis les MSI et NSIS candidats.
- [ ] Fenêtre du bureau compilée au sous-système Windows GUI ; aucune console nécessaire à sa durée de vie.
- [ ] Première authentification, signal de disponibilité complet du dashboard, redémarrage, login et logout réussis.
- [ ] Pas de fausse assertion « données non enregistrées » si le vérificateur existe déjà.
- [ ] Défaillances d'initialisation visibles et diagnostiquées par un **code borné**, sans stack ni secret.

### 03 — Données et flux

- [ ] WebSocket public limité à des hôtes autorisés, gestion TLS, timeouts, Ping/Pong et backoff borné.
- [ ] Observations réellement reçues, horodatées et attribuées à une source ; données manquantes ou périmées explicitement signalées.
- [ ] Aucun tick factice sur la vue live et aucun résultat de fixture compté comme connectivité réelle.
- [ ] Mode diagnostic hors réseau : zéro appel public ou privé, et aucun ordre.
- [ ] Tests de reconnexion, panne réseau, messages erronés, silence prolongé et arrêt propre.

### 04 — Persistance et diagnostic

- [ ] Base SQLite/WAL préservée entre installations et redémarrages.
- [ ] Aucun effacement, migration destructive ou création automatique de mot de passe au diagnostic.
- [ ] Rapport avec PASS/WARN/FAIL/SKIP et comptage cohérent ; échecs non supprimés.
- [ ] Agent de diagnostic local : une seule instance, exécution bornée de test, arrêt explicite et nettoyage des profils jetables.
- [ ] Aucun rapport ni secret transmis automatiquement à un service distant.

### 05 — Sécurité de la dépendance et du paquet

- [ ] Rust fmt, clippy, tests Windows/Linux, MSRV, CodeQL et tests IPC natifs au même SHA.
- [ ] Dépendances desktop auditées séparément ; vulnérabilités RustSec bloquantes triées et corrigées.
- [ ] MSI + NSIS install/uninstall testés sur runner Windows indépendant, pas simplement compilés.
- [ ] Empreintes SHA-256 des artefacts et manifeste de provenance vérifiés après transfert.
- [ ] Pour toute release signée : Authenticode, identité du certificat et horodatage **vérifiés**.
- [ ] Pour l'updater : signature de l'artefact, version, anti-downgrade, erreur réseau, reprise et rollback testés.
- [ ] Tests sur VM sans WebView2 et sans réseau **seulement** si la distribution revendique l'installation totalement airgap.

## Présentation d'une PR

Chaque PR contient :

| Champ | Contenu attendu |
| :--- | :--- |
| **Purpose** | Problème observé, utilisateur affecté et résultat attendu |
| **Changed** | Liste exacte des composants, permissions et dépendances modifiés |
| **Safety** | Modes désactivés, invariants IPC, secrets et périmètre financier |
| **Evidence** | Liens vers runs/jobs, commit SHA, checks réellement passés |
| **Limitations** | Tests SKIP, environnement non couvert, défauts reproductibles |
| **Decision** | READY, CHANGES REQUESTED, APPROVED ou NO-GO, avec justification |

Les revues Codex/CodeQL sont utiles mais ne remplacent pas la recette native installée. Une discussion de revue critique ne doit être marquée « resolved » qu'après correctif au SHA exact et test de non-régression.

## Règle actuelle de livraison

Au **6 novembre 2026**, la [PR #159](https://github.com/suvorovpravda/Pravda-Bot/pull/159) concentre les corrections Windows, GUI, marchés et diagnostic. L'[Issue #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160) suit les risques sécurité et déploiement. Le test MSI de la [PR #157](https://github.com/suvorovpravda/Pravda-Bot/pull/157) a révélé une initialisation de dashboard incomplète ([preuve CI](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42290188079)).

**Décision : NO-GO tant qu'un test du nouveau binaire installé ne prouve pas le chargement complet du tableau de bord et que les autres barrières de l'Issue #160 ne sont pas satisfaites.** Ne pas demander à l'utilisateur d'installer encore un autre candidat avant cette étape.

---

**EN** — Review the exact candidate, not an earlier build. A green compile does not prove a working installed desktop. Keep live trading disabled and make unresolved blockers visible.
