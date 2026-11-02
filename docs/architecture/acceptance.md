# Critères d'acceptation et stratégie de preuve — Pravda-Bot Windows Enterprise

**Statut de ce fichier :** exigences futures. **Aucun contrôle ci-dessous n'est attesté comme accompli par PR-01.**

## Barrière de fusion par PR

- [ ] Une unité de changement vérifiable par PR, aucun refactor + nouveau trading réel + changement package mêlés.
- [ ] Test de régression Rust Linux/Windows + MSRV, Clippy, fmt, dashboard contract ; branche `main` protégée.
- [ ] CodeQL et examen des alertes ouvertes ; un workflow vert n'est **pas** la preuve de zéro vulnérabilité.
- [ ] Tests de sécurité négatifs adaptés aux privilèges introduits : capacités, scénarios de rejet, ressources détournées.
- [ ] Documentation et matrice de compatibilité (ancien profil → nouvelle version) à jour.
- [ ] Revue explicite des routes, DTO, données persistantes et rollback.

## Gates d'installation (PR-08 à PR-10)

| Test | Conditions de réussite | Artefact |
|---|---|---|
| Windows vanilla | Windows 11 x64 propre, compte standard, WebView2 fourni ou bootstrap officiel | journal + captures + résultat |
| Installation silencieuse MSI | Installation sans intervention suivant la politique MSI; aucune demande d'admin à l'exécution courante | code sortie MSI + preuves système |
| Lancement normal | Un seul EXE graphique, sans CMD/PowerShell, sans console noire, sans serveur HTTP du desktop | liste processus/écoutes |
| Première utilisation | Setup et login 15+ caractères ; stockage hash salé non plaintext ; fermeture/relaunch/relogin | test E2E en WebView2 réel |
| Utilisateur existant | Base SQLite et `pravda_auth.json` conservés ; migrations sûres ; rollback sans perte | tests upgrade+backup |
| Sécurité IPC | Hors fenêtre/capability → refus ; aucun shell arbitraire ; CSP sans JS distant | matrice appels négatifs |
| Arrêt/crash | Pas d'instance orpheline, pas de base corrompue, diagnostics redacted | test injection panne |
| Désinstallation | Application supprimée ; politiques explicites pour conserver les données utilisateur | journal désinstallation |
| Authenticode | `.exe` ET `.msi` signés éditeur approuvé, SHA-256, RFC 3161 et vérification réussie | sortie signtool + manifeste |
| Supply-chain | SBOM, dépendances auditées, pipeline sans secret en logs, provenance vérifiée | SBOM + attestations |
| SmartScreen | Résultat consigné sur machine cible ; ne pas exiger « zéro avertissement » uniquement car signature | rapport manuel |
| Fonctionnement hors ligne | L'application démarre et exprime clairement « données marché indisponibles » sans crash | test réseau coupé |

## Conditions impératives de publication

1. Tous tests bloquants réussis et enregistrés pour le **SHA immuable** de la release.
2. Aucun .cmd/.ps1, serveur `pravda-server.exe`, `.env`, fichier `pravda_auth.json`, base SQLite personnelle ou certificat PFX dans le MSI produit.
3. Signatures et horodatages vérifiés après bundling ; le bundle ne doit pas altérer le binaire signé.
4. Publication empêchée si le certificat ou fournisseur de signature n'est pas configuré, si un contrôle échoue ou si un artefact a changé depuis les tests.
5. Uniquement une nouvelle release signée ; ne jamais altérer une release GitHub déjà immuable.
6. Notes de version incluant limitations fonctionnelles : simulation papier seulement, sections FX/News/AIS non migrées.

## Prérequis hors logiciel

La signature Authenticode exige un certificat/une identité d'éditeur et un mécanisme de signature réellement provisionnés. Il n'est pas possible de satisfaire cette exigence avec une clé d'updater Tauri ni en inventant un secret GitHub Actions. Le code peut être préparé, mais **la publication doit rester bloquée** jusqu'à cette étape.

L'objectif « aucune alerte SmartScreen si signé » n'est pas garanti par la signature ; la réputation et les politiques de l'entreprise doivent être évaluées indépendamment.

## Références

- https://v2.tauri.app/distribute/sign/windows/
- https://v2.tauri.app/reference/config/#frontenddist
- https://learn.microsoft.com/en-us/windows/security/application-security/application-control/windows-defender-application-control/
