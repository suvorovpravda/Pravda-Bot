<div align="center">

# 🛰️ PRAVDA-BOT
### MARKET INTELLIGENCE · WINDOWS CONTROL CENTER

**Surveillance des marchés · Simulation maîtrisée · Diagnostics natifs**

![Windows](https://img.shields.io/badge/WINDOWS-x64-4F85FF?style=for-the-badge&logo=windows)
![Rust](https://img.shields.io/badge/ENGINE-RUST-8B7CFF?style=for-the-badge&logo=rust)
![Tauri](https://img.shields.io/badge/DESKTOP-TAURI%202-705CF4?style=for-the-badge)
![SQLite](https://img.shields.io/badge/STORAGE-SQLITE-28A8E9?style=for-the-badge)
![Release](https://img.shields.io/badge/RELEASE-NO--GO-FF507F?style=for-the-badge)
![Live](https://img.shields.io/badge/LIVE%20ORDERS-OFF-3DC7AD?style=for-the-badge)

[**Dashboard**](#-control-dashboard) · [**Progression**](#-progression-historique) · [**Modules**](#-modules--état-réel) · [**Architecture**](#-architecture) · [**Security**](SECURITY.md) · [**Reviewing**](docs/REVIEWING.md)

</div>

---

## ⚡ Live Pulse — progression quasi en temps réel

![Suivi automatique GitHub : progression historique, CI au SHA de référence, PR et Issues ouvertes, publication NO-GO](https://raw.githubusercontent.com/suvorovpravda/Pravda-Bot/status/live/docs/assets/pravda-live-status.svg)

**[Voir le statut JSON vérifiable](https://github.com/suvorovpravda/Pravda-Bot/blob/status/live/docs/assets/pravda-live-status.json)** · **[Historique des actualisations GitHub](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/live-docs.yml)**

Ce panneau est **alimenté par la branche dédiée `status/live`** environ toutes les 10 minutes et après les workflows CI surveillés, selon la disponibilité de GitHub Actions. **La branche `main` reste protégée** : aucune mise à jour automatique ne la contourne. Les textes permanents du README et de SECURITY sont modifiés via Pull Request, tandis que ce panneau et son JSON reflètent les dernières preuves GitHub publiées. Le cache des images GitHub peut retarder leur affichage. Aucune exécution CI ne vaut certification de production.

**Source de vérité :** derniers runs CI sur `main` et leur SHA, nombre de PR/Issues ouvertes, et jalons chiffrés du guide central. Les audits RustSec **non bloquants** affichent `OBSERVE ONLY`, jamais un feu vert de sécurité. **Un workflow PASS n'est pas un GO production.** La dernière preuve affichée est horodatée ; si la synchronisation échoue, les données restent celles de la dernière réussite et ne doivent pas être considérées comme instantanées.

## 🌌 Control dashboard

![Cockpit néon Pravda-Bot : jauges techniques historiques, neuf domaines de progression, décision de publication NO-GO et barrières de release](docs/assets/pravda-progress-cockpit.svg)

> **Lecture des chiffres.** Les jauges représentent la **baseline historique du guide technique**, et non un taux de disponibilité du produit. L'indicateur global **47 %** correspond à **20 jalons sur 43** dans les neuf domaines historiques. Il **ne signifie pas que Pravda-Bot fonctionne à 47 %**.

<div align="center">

[![Rust CI](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml)
[![CodeQL](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml)
[![Windows Packaging](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml/badge.svg?branch=main)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml)

![Monitoring](https://img.shields.io/badge/MARKET%20DATA-VALIDATION%20REQUIRED-FFB656?style=flat-square)
![Diagnostics](https://img.shields.io/badge/DIAGNOSTICS-LOCAL-32D5F6?style=flat-square)
![Paper](https://img.shields.io/badge/PAPER%20TRADING-ISOLATED-A28EFF?style=flat-square)
![Updater](https://img.shields.io/badge/AUTO%20UPDATER-OFF-FF4D8F?style=flat-square)
![Signing](https://img.shields.io/badge/ENTERPRISE%20SIGNING-NOT%20QUALIFIED-FF6B82?style=flat-square)

</div>

Les badges CI reflètent les workflows de **main** au moment de l'affichage. Ils ne prouvent **ni** que l'installation Windows marche sur le PC cible, **ni** une certification de sécurité.

## 📊 Progression historique

**Neuf domaines P0–P2, chiffres du [guide central](docs/PRAVDA-GUIDE.md), non assimilables à un score de sécurité ou à une préparation à la production.**

| Priorité / domaine | Progression | Barres visuelles | Limite |
| :--- | ---: | :--- | :--- |
| 🟣 P0 · Séparation Python → Rust | **100 %** | 🟪🟪🟪🟪🟪🟪🟪🟪🟪🟪 | Migration du périmètre actif |
| 🔵 P0 · Sécurité IPC | **50 %** | 🟦🟦🟦🟦🟦⬛⬛⬛⬛⬛ | Contrôles restants |
| 🟣 P0 · Parité des fonctions Rust | **38 %** | 🟪🟪🟪🟪⬛⬛⬛⬛⬛⬛ | Fonctions à compléter |
| 🔵 P0 · CI et gouvernance | **60 %** | 🟦🟦🟦🟦🟦🟦⬛⬛⬛⬛ | Recette release distincte |
| 🟢 P1 · SQLite / historique | **75 %** | 🟩🟩🟩🟩🟩🟩🟩🟩⬛⬛ | Restauration à qualifier |
| 🌸 P1 · Interface desktop | **50 %** | 🟪🟪🟪🟪🟪⬛⬛⬛⬛⬛ | Jalon historique ; MSI/NSIS E2E validés depuis, voir #168 |
| 🔴 P1 · Packaging Enterprise | **0 %** | ⬛⬛⬛⬛⬛⬛⬛⬛⬛⬛ | Signature non livrée |
| 🟠 P2 · Observabilité / charge | **25 %** | 🟧🟧🟧⬛⬛⬛⬛⬛⬛⬛ | Extension à faire |
| ⚪ P2 · Sources et modules futurs | **0 %** | ⬛⬛⬛⬛⬛⬛⬛⬛⬛⬛ | Hors chemin critique |

<details>
<summary><strong>📈 Déplier le graphique de progression des neuf domaines</strong></summary>

~~~mermaid
xychart-beta
    title "Jalons historiques — non assimilables à la readiness"
    x-axis ["Rust","IPC","Parite","CI","SQLite","Desktop","Signing","Observ.","Extras"]
    y-axis "Jalons (%)" 0 --> 100
    bar [100,50,38,60,75,50,0,25,0]
~~~

</details>

**Agrégation pondérée des jalons historiques :** P0 **61 %** (14/23), P1 **42 %** (5/12), P2 **13 %** (1/8). La décision de publication dépend **exclusivement** des critères GO / NO-GO ci-dessous.

## 🚦 Release gate — état actuel

| Vérification | État | Preuve manquante |
| :--- | :---: | :--- |
| 🖥️ Bureau Windows installé | ✅ **E2E CI VALIDÉ** | MSI et NSIS : inscription, JS complet, reconnexion, déconnexion ; qualification poste utilisateur distincte |
| 📦 MSI / NSIS | ✅ **INSTALLATION CI VALIDÉE** | Installation, suppression et conservation SQLite en runner Windows ; signature et VM airgap non qualifiées |
| 📡 Flux Kraken / Binance | 🟠 **NON PROUVÉS SUR PC** | Cotations réelles horodatées, provenance et reconnexion |
| 🧪 Doctor / Harness | 🔵 **OUTILS DE TEST** | Les scénarios synthétiques ne prouvent pas les cours live |
| 🛡️ RustSec desktop | 🔴 **À TRAITER** | Vulnérabilités et dépendances historiques |
| 🔏 Signature Enterprise | 🔴 **NON LIVRÉE** | Authenticode, identité, timestamp |
| 🔄 Updater automatique | ⚪ **OFF** | Signature, anti-downgrade, rollback |
| 💳 Ordres réels | 🟢 **OFF** | Aucun trading live activé |

**Incident Windows résolu en recette CI :** les précédents débordements JavaScript `P02H/P03 · STACK_OVERFLOW` empêchaient l'initialisation du tableau de bord. La [PR #168](https://github.com/suvorovpravda/Pravda-Bot/pull/168) a été fusionnée sur `main` ; le [run Windows indépendant 42349767279](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42349767279), sur le commit `1021115e`, confirme l'initialisation graphique MSI **et** NSIS, le mot de passe, la session Rust, le redémarrage, la reconnexion, la déconnexion, la désinstallation et la conservation des données. **Cette recette ne prouve ni la compatibilité avec tous les postes utilisateurs, ni la signature Enterprise, ni des ordres live.**

L'installation Tauri utilise la configuration WebView2 **offlineInstaller**, mais son fonctionnement sur une VM vierge réellement isolée du réseau n'est **pas encore attesté**.

**Échéance cible : 30 novembre 2026**, sous réserve des preuves de l'[Issue #160 — Security / Déploiement](https://github.com/suvorovpravda/Pravda-Bot/issues/160).

## 🧩 Modules & état réel

| Composant | Situation | Rôle |
| :--- | :--- | :--- |
| **Market Radar** | 🟠 Validation | Prix publics observés, historique, graphiques |
| **Data Sources** | 🟠 Validation | Provenance et fraîcheur |
| **Paper Trading** | 🟣 Simulation | Exécution fictive isolée des comptes réels |
| **Risk Engine** | 🟠 Progressif | Contrôles et refus des données périmées |
| **Local Storage** | 🟢 Fondations | SQLite / WAL, intégrité locale |
| **System Doctor** | 🔵 Diagnostic | Contrôles Windows passifs sans extraction de secrets |
| **Continuous Test Agent** | 🟠 Recette | Contrôles périodiques isolés, rapport local |
| **Signed Updater** | 🔴 Désactivé | Aucune mise à jour signée homologuée |

**Aucun prix fictif ne doit apparaître comme une observation réelle.** Doctor et Harness ne prétendent pas tester la connectivité externe.

## 🧬 Architecture

~~~mermaid
flowchart LR
    UI["🖥️ Tauri / WebView2"]
    IPC["🔐 IPC session Rust"]
    CORE["⚙️ Rust Runtime"]
    WSS["📡 Kraken / Binance"]
    SQL[("🗄️ SQLite / WAL")]
    PAPER["🧪 Paper Simulation"]
    DIAG["🛡️ Doctor / Harness"]
    UI --> IPC --> CORE
    WSS --> CORE
    CORE --> SQL
    CORE --> PAPER
    CORE --> DIAG
~~~

L'interface ne reçoit pas les secrets, ne peut pas soumettre des ordres réels et n'exige pas de serveur HTTP local. Les modules optionnels sont contrôlés côté Rust.

## 🗺️ Roadmap

- [x] **P0** · Runtime Windows Rust / Tauri.
- [x] **P0** · Intégration des correctifs GUI, marché et diagnostics via #159.
- [x] **P0** · Recette graphique MSI et NSIS sur un runner Windows indépendant ([run 42349767279](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42349767279)).
- [ ] **P0** · Flux publics réellement observés et diagnostic des pannes réseau.
- [ ] **P0** · RustSec desktop et barrières de provenance.
- [ ] **P1** · Tests airgap sur VM vierge et installation réellement personnalisée.
- [ ] **P1** · Chaîne Authenticode et mises à jour signées qualifiées.
- [ ] **P2** · Extensions on-chain, simulation avancée et observabilité.

Le trading réel [#89](https://github.com/suvorovpravda/Pravda-Bot/issues/89), l'on-chain [#121](https://github.com/suvorovpravda/Pravda-Bot/issues/121) et le moteur événementiel [#123](https://github.com/suvorovpravda/Pravda-Bot/issues/123) restent des chantiers indépendants, non déclarés livrés.

## 📚 Documentation

[![SECURITY](https://img.shields.io/badge/SECURITY-TRUST%20CONTROL-FF4D8F?style=for-the-badge)](SECURITY.md)
[![REVIEWING](https://img.shields.io/badge/REVIEWING-EVIDENCE--FIRST-967CFF?style=for-the-badge)](docs/REVIEWING.md)
[![GUIDE](https://img.shields.io/badge/TECHNICAL%20GUIDE-RUST-37BFEF?style=for-the-badge)](docs/PRAVDA-GUIDE.md)
[![RELEASE](https://img.shields.io/badge/RELEASE%20GATE-ISSUE%20%23160-FB696B?style=for-the-badge)](https://github.com/suvorovpravda/Pravda-Bot/issues/160)

Le projet reste **NO-GO** pour une production complète : la recette d'installation Windows de la PR #168 est réussie, mais la signature, l'audit des dépendances, les sources réelles, l'updater et la qualification de publication restent des barrières indépendantes.

---

**EN** — Windows market monitoring and paper simulation on Rust/Tauri 2. Progress percentages are **historical engineering milestones**, not release readiness. Production NO-GO; live trading OFF.

**RU** — Мониторинг рынков и симуляция для Windows на Rust/Tauri 2. Проценты — **исторические этапы разработки**, не готовность к релизу. Продакшен NO-GO, реальные ордера отключены.
