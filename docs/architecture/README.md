# Architecture cible Windows — Pravda-Bot

**Statut :** RFC-0001 en revue — documentation de décision uniquement, aucune fonctionnalité implémentée par cette PR.  
**Référence auditée :** `main` au commit `423ab09c85228466d90a23269d903db3415df0c6` (2026-11-02).  
**Application :** Rust + Tauri 2 + SQLite, Windows x64.

Documents :
- [RFC-0001 : architecture et séquencement](RFC-0001-windows-enterprise.md)
- [Modèle de menace et frontières de confiance](threat-model.md)
- [Inventaire réel des routes et contrat de migration UI → IPC](ui-ipc-contract.md)
- [Critères d'acceptation et contrôle de release](acceptance.md)
- [Tâche 3 — migration SQLite legacy transactionnelle](sqlite-legacy-migration.md)

## Invariants

1. `pravda-app.exe` est l'unique entrée du desktop **production** ; le moteur Rust tourne dans le processus de l'application, sans script shell.
2. UI embarquée (`frontendDist`) et communication locale via **IPC Tauri** ; aucune écoute TCP requise par le desktop. Le serveur Axum reste une **cible de diagnostic distincte**, exclue du MSI.
3. Permissions minimales par fenêtre **et** restriction explicite des commandes personnalisées ; commandes métiers validant elles-mêmes authentification et autorisation.
4. SQLite et l'authentification existante ne sont ni supprimés ni réinitialisés par les migrations ; évolution atomique et retour arrière documenté.
5. Un paquet destiné à la production exige binaire et MSI **signés Authenticode**, horodatage RFC 3161 et vérification d'intégrité en CI. En l'absence de certificat/accès au service de signature approuvés : **pas de publication enterprise**.
6. Aucun contournement de contrôle Host/Origin, de politique Windows ou de défaut de certificat pour « faire fonctionner » l'interface.
7. La sortie Release est graphique (pas de console), avec erreurs consultables sans divulguer des secrets ; pas de script CMD/PS1 dans le produit livré.

## Portée actuelle

Le travail PR-01 est strictement documentaire : il établit les contrats, la gouvernance et les critères mesurables avant modification du code. La PR de dépannage HTTP [#48](https://github.com/suvorovpravda/Pravda-Bot/pull/48) est une branche parallèle en revue, **pas** la preuve de l'architecture cible.

## Références éditeur

- Tauri v2 — [structure](https://v2.tauri.app/start/project-structure/), [assets embarqués](https://v2.tauri.app/reference/config/#frontenddist), [permissions et capacités](https://v2.tauri.app/reference/acl/permission/), [code signing Windows](https://v2.tauri.app/distribute/sign/windows/)
- NIST SP 800-207 — [Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final)
