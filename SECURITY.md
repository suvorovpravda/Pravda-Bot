# Sécurité de Pravda-Bot

Pravda-Bot est une application desktop Windows open source dédiée au suivi de marché et à la simulation. La sécurité est traitée comme une propriété de l’architecture, du code, des dépendances et de chaque livraison.

> **Périmètre financier :** les fonctions décrites dans le dépôt couvrent la consultation et la simulation. Les ordres de trading réel sont désactivés.

## Principes de conception

- **Traitements applicatifs en Rust.** L’interface Tauri communique avec le cœur par les commandes IPC prévues par l’application.
- **Séparation des usages.** Observation des marchés, données locales, évaluations de risque et simulation papier ont des rôles distincts.
- **Stockage local.** L’historique pris en charge est conservé dans SQLite/WAL sur le poste.
- **Diagnostics maîtrisés.** Doctor et Harness servent à la vérification et à la recette ; ils ne sont pas des outils de trading.
- **Contrôles continus.** Le dépôt utilise des workflows CI, CodeQL et des contrôles de dépendances. Leurs résultats sont consultables dans l’onglet Actions.

Ces choix réduisent certaines surfaces d’exposition et rendent les contrôles examinables. Ils ne rendent pas un poste invulnérable et ne remplacent ni la protection de Windows ni la vérification des fichiers téléchargés.

## Points suivis avant publication

La publication générale reste **NO-GO** tant que les critères de l’[Issue #160 — sécurité et déploiement](https://github.com/suvorovpravda/Pravda-Bot/issues/160) ne sont pas vérifiés sur les artefacts candidats.

**Point d'étape dépendances (branche `main`, commit [`bfdadd75063cf600015fa8e263cb00aa1c42c09b`](https://github.com/suvorovpravda/Pravda-Bot/commit/bfdadd75063cf600015fa8e263cb00aa1c42c09b), 13 novembre 2026) :** les deux fichiers `Cargo.lock` (`/Cargo.lock` et `crates/pravda-app/Cargo.lock`) contiennent `reqwest 0.13.5`, `hyper 1.12.0` et `h2 0.4.20`. L'ancienne chaîne `reqwest 0.11.27 → hyper 0.14.32 → h2 0.3.27` ne figure plus dans ces lockfiles. Pour **RUSTSEC-2026-0258**, le seuil de correction annoncé est `h2 >= 0.4.16` : la version verrouillée `0.4.20` dépasse donc ce seuil **pour cet avis précis**. Cela ne constitue ni un résultat `cargo audit` exécuté sur le futur candidat, ni une conclusion sur les autres avis RustSec. L'inventaire doit être réévalué sur le **SHA exact de chaque build** (deux workspaces, features de l'updater comprises), avec `cargo audit` bloquant sans dérogation silencieuse. La preuve et les contrôles encore ouverts sont suivis dans l'[issue #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160).

Le packaging Windows configure le prérequis WebView2 avec `offlineInstaller`. Cette configuration décrit le mode prévu ; elle ne certifie pas une installation complète sur une machine vierge hors réseau.

## Vérifications visibles

![État horodaté des contrôles GitHub et du jalon de publication](https://raw.githubusercontent.com/suvorovpravda/Pravda-Bot/status/live/docs/assets/pravda-live-status.svg)

[Statut JSON et date de synchronisation](https://github.com/suvorovpravda/Pravda-Bot/blob/status/live/docs/assets/pravda-live-status.json) · [Historique des workflows](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/live-docs.yml)

[![Évaluation de sécurité SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=suvorovpravda_Pravda-Bot&metric=security_rating)](https://sonarcloud.io/project/overview?id=suvorovpravda_Pravda-Bot)

Le panneau reflète les vérifications publiées pour le SHA affiché ; la date JSON permet de juger leur fraîcheur. Les contrôles automatisés sont une partie du processus de sécurité, pas une certification de l’application.

## Limites à connaître

- Un résultat CI valide le contrôle exécuté sur le commit indiqué ; il ne certifie pas à lui seul l’application complète ni le poste de l’utilisateur.
- Une compilation ou une recette de paquet ne signifie pas que l’installateur est signé ou qu’un mécanisme de mise à jour est qualifié. Consultez les [versions](https://github.com/suvorovpravda/Pravda-Bot/releases) et leurs preuves.
- La disponibilité et la fraîcheur des cours dépendent des fournisseurs et du réseau.
- La simulation utilise des opérations fictives et ne prédit pas les résultats futurs.
- Le dépôt ne revendique aucune inviolabilité, certification réglementaire ou garantie de sécurité absolue.

## Signaler une vulnérabilité

**Merci de ne pas publier de vulnérabilité exploitable, d’identifiant de compte ou de secret dans une Issue ou une Pull Request publique.**

Utilisez en priorité la fonction **Report a vulnerability** dans l’onglet [Security du dépôt](https://github.com/suvorovpravda/Pravda-Bot/security), lorsqu’elle est disponible. Si GitHub ne propose pas ce formulaire, contactez le mainteneur par un canal privé indiqué sur son [profil GitHub](https://github.com/suvorovpravda) et demandez un canal de signalement sécurisé.

Incluez, si possible :

- la version ou le SHA concerné ;
- le composant et les conditions nécessaires ;
- l’impact observé et les étapes minimales de reproduction ;
- une preuve expurgée qui ne contient ni clé, ni token, ni donnée personnelle.

Ne transmettez pas de mot de passe, clé API, token, fichier PFX, base SQLite, fichiers WAL/SHM, export de compte ou journaux contenant des données privées.

## Traitement des signalements

Le mainteneur évaluera la portée et la reproductibilité, puis suivra les étapes adaptées : confirmation, correction, validation, avis de sécurité et publication. Le délai dépend de la gravité, de la complexité et de la disponibilité d’un correctif sûr ; aucun délai de réponse fixe n’est promis.

Les signalements coordonnés sont appréciés. Attendez-vous à un accord sur la divulgation publique avant de publier des détails permettant l’exploitation.

## Versions et état des correctifs

Les correctifs de sécurité sont publiés dans le dépôt et associés aux changements concernés. Pour toute vérification, relevez le SHA de la branche ou de la version installée et consultez les changements ainsi que les contrôles CI correspondants. Une version plus ancienne peut ne pas inclure les corrections récentes.

## Liens utiles

- [README — présentation de Pravda-Bot](README.md)
- [Guide central — architecture et validation](docs/PRAVDA-GUIDE.md)
- [Actions — résultats CI](https://github.com/suvorovpravda/Pravda-Bot/actions)
- [Versions publiées](https://github.com/suvorovpravda/Pravda-Bot/releases)
- [Créer un signalement privé](https://github.com/suvorovpravda/Pravda-Bot/security/advisories/new)
