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
