<div align="center">

# PRAVDA / BOT

### Le marché, sans le bruit.

**Observatoire de marchés · Simulation sécurisée · Diagnostics Windows**

Windows x64 &nbsp; / &nbsp; Rust + Tauri 2 &nbsp; / &nbsp; SQLite local

[Le produit](#le-produit) &nbsp;·&nbsp; [Livraison](#état-de-livraison) &nbsp;·&nbsp; [Architecture](#architecture) &nbsp;·&nbsp; [Sécurité](SECURITY.md) &nbsp;·&nbsp; [Reviewing](docs/REVIEWING.md)

</div>

---

## Le produit

**Un seul espace pour observer les marchés, interpréter les données et maîtriser les risques.**

Pravda-Bot réunit cotations publiques, graphiques, provenance des sources, alertes, simulation et outils de contrôle. Une valeur absente n'est jamais remplacée par un prix inventé. La fraîcheur et la disponibilité doivent être visibles.

| Espace | Fonction | Limite |
| :--- | :--- | :--- |
| **Marchés** | Cotations Kraken/Binance, graphiques et historique | Connexions et prix réels à confirmer, jamais simulés comme réels |
| **Sources** | Références publiques, macroéconomie et actualité | Provenance, fraîcheur et consentement |
| **Simulation** | Paper trading et scénarios de risque | Aucun ordre financier réel |
| **Système** | CPU, mémoire, disque, état SQLite | Diagnostics locaux, aucun secret vers la WebView |
| **Sécurité** | Sessions Rust, contrôles IPC, rapports | FAIL/WARN/SKIP explicites ; aucun succès fictif |

L'application Windows repose sur **Tauri 2 / WebView2**, avec un cœur **Rust** et une base locale **SQLite/WAL**. Les anciennes sources Python sont archivées : elles ne font pas partie du runtime Windows.

## État de livraison

**06 novembre 2026 · NO-GO production.** L'objectif reste le **30 novembre 2026**, mais la publication dépend des preuves de recette.

> **Défaut réel :** lors du test de l'application installée, la création du mot de passe réussit puis le tableau de bord ne finit pas son initialisation. Le job Windows de la [PR #157](https://github.com/suvorovpravda/Pravda-Bot/pull/157) a échoué avec le diagnostic « Desktop startup reported a partial dashboard initialization » ([run 42290188079](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42290188079)). Une compilation verte n'est pas une validation de cette interface.

| Capacité | Situation actuelle |
| :--- | :--- |
| Rust, Tauri et stockage SQLite | Fondations présentes ; tests de composants |
| Installeurs Windows MSI / NSIS | Construits en CI ; **recette intégrée finale non acquise** |
| Authentification et GUI installée | **Blocage confirmé** sur le parcours de première connexion |
| Bureau sans terminal attaché | Correction intégrée à la PR #159 ; package final à valider |
| Flux publics Binance/Kraken | Reconnexion et état des flux retravaillés ; observations réelles sur PC non vérifiées |
| Doctor, Harness, diagnostic continu | Diagnostics locaux et essais isolés ; ne prouvent pas la connectivité publique |
| VM vierge sans réseau / WebView2 | Non homologuée |
| Installation personnalisée | Non homologuée |
| Signature Enterprise et auto-updater | Non déployés en production |
| Trading réel Revolut X et retraits | **Désactivés** |

### Un seul chantier P0

**[PR #159 — intégration Windows, marchés et diagnostics](https://github.com/suvorovpravda/Pravda-Bot/pull/159)** : branche unique regroupant les anciennes PR #156, #157 et #158. Elles ont été fermées sans fusion séparée afin d'éviter des validations contradictoires. La PR #153 de vérification cryptographique des candidats a été fusionnée, mais **n'active pas** les mises à jour automatiques.

**[Issue #160 — Security / Déploiement : GO / NO-GO](https://github.com/suvorovpravda/Pravda-Bot/issues/160)** : registre des obstacles restants, des preuves et des vérifications préalables à toute diffusion.

Les trois extensions P1 [trading réel #89](https://github.com/suvorovpravda/Pravda-Bot/issues/89), [on-chain #121](https://github.com/suvorovpravda/Pravda-Bot/issues/121) et [moteur événementiel #123](https://github.com/suvorovpravda/Pravda-Bot/issues/123) restent ouvertes parce qu'elles ne sont **pas** achevées.

### Distribution

**Aucun nouvel installeur ne doit être présenté comme terminé** tant que la PR #159 n'a pas passé les tests nécessaires au même SHA : installation MSI/NSIS, création du mot de passe, tableau de bord initialisé, redémarrage, reconnexion, diagnostic et conservation des données.

Les archives GitHub Actions sont des candidats temporaires ; une archive non signée n'est pas une release Enterprise. La configuration MSI/NSIS prévoit le prérequis WebView2 embarqué en mode `offlineInstaller`, **sans constituer une preuve de recette VM sans réseau**. Ne jamais désactiver Windows Defender, SmartScreen ou l'UAC pour contourner une alerte.

## Utilisation visée

1. **Installer** : un programme Windows autonome, sans exiger Python, Node ou Rust sur le poste.
2. **Ouvrir** : une vraie fenêtre graphique, sans console PowerShell dont la fermeture tue l'application.
3. **S'authentifier** : vérification locale par Rust ; la WebView ne reçoit aucun secret du coffre.
4. **Observer** : flux autorisés, valeurs réellement reçues, source et fraîcheur.
5. **Diagnostiquer** : lancer un test sûr et copier le résultat expurgé.

**Ce parcours est un critère de validation, pas une fonction entièrement homologuée à ce jour.**

Le Doctor inspecte passivement l'environnement et l'intégrité du stockage. Le Harness utilise des données **synthétiques**, clairement distinctes des vrais flux. Le centre de test continu en cours de validation écrit des rapports locaux dans le dossier utilisateur Windows et ne les envoie pas automatiquement. Ni Doctor ni Harness ne prouvent à eux seuls le bon fonctionnement du tableau de bord ou des exchanges.

## Architecture

| Couche | Technologie | Frontière de responsabilité |
| :--- | :--- | :--- |
| Interface | Tauri 2, WebView2, HTML/CSS/JS locaux | Affichage, consentement et contrôle visuel |
| Autorisation | IPC Tauri + sessions Rust | Permissions par commande et révocation |
| Données | Collecteurs WebSocket Rust, Tokio | Provenance, fraîcheur, reconnexion, refus |
| Exécution | Moteur Rust | Simulation isolée et règles de risque |
| Persistance | SQLite/WAL | Historique, cohérence transactionnelle |
| Tests | Doctor, Harness et WebDriver Windows | Contrôles passifs, fixtures et E2E du binaire installé |
| Livraison | MSI / NSIS et bootstrapper | Empreintes, vérification et signature si applicable |

Aucun serveur HTTP local n'est nécessaire au bureau Tauri. La lecture publique de données financières ne confère pas le droit d'émettre des ordres.

## Barrières de validation

- [ ] Compilation et formatage Rust, clippy, tests Windows/Linux, CodeQL au SHA exact.
- [ ] Audit RustSec **du Cargo.lock desktop** traité et signaux de vulnérabilité résolus ou formellement bloquants.
- [ ] Sous-système Windows GUI vérifié sur le binaire effectivement distribué.
- [ ] MSI **et** NSIS installés, première authentification, initialisation totale du dashboard et redémarrage validés en E2E.
- [ ] Diagnostic continu, arrêt explicite et rapports sans données personnelles vérifiés.
- [ ] SHA-256/provenance établis ; Authenticode vérifié pour toute release qui se prétend signée.
- [ ] Flux publics réels contrôlés séparément et **avec consentement**, sans accès privé.
- [ ] Absence de trading réel et d'updater actif non validé.

Un test SKIP ne vaut pas PASS. Voir les règles de [reviewing](docs/REVIEWING.md) et le [registre #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160).

## Documentation

[Security — modèle de menace, risques et signalement](SECURITY.md) &nbsp;·&nbsp; [Reviewing — exigences de revue et de publication](docs/REVIEWING.md) &nbsp;·&nbsp; [Guide central — architecture et historique](docs/PRAVDA-GUIDE.md) &nbsp;·&nbsp; [Workflows CI](https://github.com/suvorovpravda/Pravda-Bot/actions)

<details>
<summary>Indicateurs historiques P0–P2</summary>

Les pourcentages historiques de migration sont conservés dans le [guide central](docs/PRAVDA-GUIDE.md) uniquement à titre de référence. Ils ne constituent pas un score actuel de sécurité ni une preuve de livraison.

</details>

---

**EN** — Pravda-Bot is a Windows-native Rust/Tauri 2 market monitoring and paper-simulation application. **Release: NO-GO** pending installed-GUI, MSI/NSIS, security and genuine feed validation. Live trading is disabled. [Security](SECURITY.md) / [Reviewing](docs/REVIEWING.md).

**RU** — Pravda-Bot — приложение для Windows на Rust/Tauri 2 для наблюдения за рынком и симуляции. **Релиз пока не разрешён** до успешной проверки установленного интерфейса, MSI/NSIS и безопасности. Реальная торговля отключена.
