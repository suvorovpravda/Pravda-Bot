<div align="center">

# 🛰️ PRAVDA-BOT
### Observatoire Windows · Rust natif · Simulation et diagnostics

![Windows](https://img.shields.io/badge/Windows-Test%20x64-2563eb?style=for-the-badge)
![Rust](https://img.shields.io/badge/Moteur-100%25%20Rust-7c3aed?style=for-the-badge)
![Trading](https://img.shields.io/badge/Trading%20réel-DÉSACTIVÉ-16a34a?style=for-the-badge)
![Enterprise](https://img.shields.io/badge/Signature-À%20VALIDER-dc2626?style=for-the-badge)

[![Rust CI](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/rust.yml)
[![CodeQL](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/codeql.yml)
[![Packaging Windows](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml/badge.svg)](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml)

**[⬇️ Télécharger](#-téléchargements-windows)** · **[🧰 Tester en un clic](#-centre-de-test-unifié)** · **[🛡️ Sécurité](SECURITY.md)** · **[📚 Guide](docs/PRAVDA-GUIDE.md)**

</div>

---

## 🇫🇷 Tableau de bord de livraison

**Objectif : 30 novembre 2026.** Les résultats verts de CI prouvent uniquement leurs propres assertions sur un commit exact. Ils ne remplacent ni un essai sur PC propre, ni une vérification hors réseau, ni une signature Enterprise.

| Capacité | État | Preuve / limite |
| :--- | :---: | :--- |
| Interface Windows native Tauri 2 / WebView2 | 🟢 Intégrée | Session Rust/IPC, données locales SQLite |
| Rust / tests / CodeQL | 🟢 CI disponible | [Workflows GitHub](https://github.com/suvorovpravda/Pravda-Bot/actions) ; vérifier le SHA |
| Installation MSI + NSIS | 🟢 Testée sur runner | Contrôles de démarrage, désinstallation et persistance |
| Diagnostic Doctor et simulation Harness | 🟢 Compilés | Compagnons autonomes, sans ordres réels |
| **Un seul EXE qui lance les trois tests** | 🟡 En revue | [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138), pas encore homologuée |
| Installation totalement hors réseau sans WebView2 initial | 🟠 Non prouvée | Recette VM indépendante requise |
| Installation Standard / Personnalisée | 🟠 À valider | Retrait des modules facultatifs côté Rust |
| Enterprise signée / horodatée | 🔴 Non livrée | [Issue #90](https://github.com/suvorovpravda/Pravda-Bot/issues/90) |
| Trading réel / retraits | 🔒 Désactivés | [Issue #89](https://github.com/suvorovpravda/Pravda-Bot/issues/89) |

![P0](https://img.shields.io/badge/P0-Recette%20Windows%20en%20cours-f59e0b)
![Plan](https://img.shields.io/badge/Plan%20initial%20%23106-Clôturé%20administrativement-64748b)
![GO](https://img.shields.io/badge/Production%20complète-NO--GO-dc2626)

**Point de vérité actuel : [issue #134 — recette Windows finale](https://github.com/suvorovpravda/Pravda-Bot/issues/134).** L'ancienne issue #106 est clôturée, mais cette clôture n'atteste pas les scénarios P0 restants.

### ⬇️ Téléchargements Windows

Le dernier run de référence vérifié le **5 novembre 2026** sur la branche principale est le [packaging #41972245070](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/41972245070), lié au commit **b2fb618b53408b0fd5fd2c4265cdfe0a05ffc898**. Rust, CodeQL, MSI et NSIS ont réussi sur ce commit. Toute version ultérieure doit être vérifiée séparément.

| Archive GitHub Actions | Contenu | Utilisation |
| :--- | :--- | :--- |
| **pravda-windows-unsigned-candidate-installers** | MSI, NSIS, EXE, sommes SHA-256 | Installation Windows de test |
| **pravda-windows-troubleshooting** | Doctor Windows | Double-clic, diagnostic passif |
| **pravda-windows-production-test** | Harness Rust | Scénarios de simulation synthétiques |
| **pravda-windows-one-click-test-center** | Centre de test tout-en-un | **Prévu en PR #138, à télécharger seulement après CI verte** |

Pour récupérer une version, ouvrir [Windows installer packaging](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml), choisir un run **réussi sur le SHA souhaité**, descendre à **Artifacts**, puis extraire l'archive.

Les paquets MSI/NSIS configurent le prérequis officiel WebView2 avec **offlineInstaller** et installation silencieuse. Aucune installation manuelle de Python, Rust, Node ou MSVC n'est requise pour utiliser les paquets. L'essai sur VM Windows réellement sans WebView2 et sans Internet reste obligatoire avant de déclarer cette autonomie homologuée.

**Important :** une archive de test non signée peut déclencher un avertissement SmartScreen. Ne jamais désactiver SmartScreen, l'antivirus, le pare-feu ou l'UAC pour contourner une protection. Les archives GitHub Actions sont temporaires et ne constituent pas une Release Enterprise.

### 🧰 Centre de test unifié

![Rapport](https://img.shields.io/badge/Rapport-HTML%20coloré%20%2B%20JSON-7c3aed)
![Confidentialité](https://img.shields.io/badge/Diagnostic-Local-16a34a)
![Exécution](https://img.shields.io/badge/Utilisateur-Un%20double--clic-2563eb)

La [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138) développe un **seul exécutable Windows autonome** embarquant les trois composants : interface, Doctor et Harness. Il doit déclencher les vérifications, ouvrir un rapport HTML lisible et générer un JSON détaillé destiné au dépannage.

**Statut actuel : en compilation/revue, pas encore un livrable validé.** Les tests qui exigent une VM réellement isolée, un installer personnalisé ou une signature Enterprise resteront **NON VÉRIFIÉS**, jamais déclarés réussis automatiquement.

Le programme n'envoie pas le rapport vers un service externe. L'utilisateur doit examiner le JSON avant tout partage ; ne jamais transmettre mot de passe, clé privée, base SQLite ou historique financier.

### 🏗️ Architecture actuelle

| Niveau | Technologies | Contrôles |
| :--- | :--- | :--- |
| Desktop | Tauri 2, WebView2, HTML/CSS/JS locaux | CSP restrictive, fenêtre principale |
| Transport | Commandes IPC nommées | Authorization Rust et session liée aux requêtes |
| Noyau | Rust | Authentification, décision de confiance, accès données |
| Persistance | SQLite / WAL | Écritures transactionnelles et réouverture |
| Doctor | Rust Windows | Rapport local expurgé, sans réparation |
| Harness | Tokio / Rust | Simulation papier, refus, crash, timeout |
| Accès financier | Lecture seule / paper | Aucun ordre réel sans autorisation distincte |

Les sources Python historiques sont archivées hors du périmètre de production. La recherche on-chain, les modèles de risque avancés et le trading réel restent distincts des fonctions déjà livrées. Voir le [guide central](docs/PRAVDA-GUIDE.md).

### 🪟 Démarrage rapide — Windows

Pour un **utilisateur final**, télécharger le MSI ou le NSIS du run validé, l'ouvrir normalement, puis créer le mot de passe local requis. Il n'est pas nécessaire de compiler le projet ni d'installer une chaîne de développement.

Pour un **développeur**, les commandes Cargo, les tests Windows, les profils et les procédures de reprise sont décrits dans le [guide technique](docs/PRAVDA-GUIDE.md).

Les données locales et les fichiers d'authentification ne doivent pas être effacés en cas d'erreur d'interface. Le bureau Tauri utilise l'IPC natif : il n'exige pas de serveur HTTP local pour afficher sa fenêtre.

### 🔐 Modèle de sécurité, en résumé

![Session](https://img.shields.io/badge/Session-Rust-16a34a)
![Coffre](https://img.shields.io/badge/Secrets-Hors%20WebView2-0e7490)
![Audit](https://img.shields.io/badge/Audit%20indépendant-Incomplet-f59e0b)

L'interface ne reçoit pas les secrets du coffre Windows. Une commande sensible requiert les permissions Tauri et la session Rust appropriée. Les demandes asynchrones ne doivent pas conserver l'autorisation d'une session déconnectée. Une CI verte ne prouve ni l'absence totale de vulnérabilités, ni la conformité à une certification financière.

[Lire la politique SECURITY.md](SECURITY.md) pour les preuves, risques, règles de publication et modalités de signalement.

<details>
<summary><strong>📊 Baseline historique des neuf indicateurs P0–P2 (afficher)</strong></summary>

Les fractions suivantes sont conservées pour la cohérence des trois documents de référence. **Elles ne sont pas un score de sécurité actuel** et les jalons nécessitent une requalification avant tout changement.

| Priorité | Chantier | Jalons | Proportion |
| :--- | :--- | :--- | :--- |
| **P0** | Séparation Python / Rust | 6/6 | **100 %** |
| **P0** | Sécurité IPC Windows | 2/4 | **50 %** |
| **P0** | Parité fonctionnelle Rust | 3/8 | **38 %** |
| **P0** | CI / gouvernance | 3/5 | **60 %** |
| **P1** | Historique SQLite | 3/4 | **75 %** |
| **P1** | Expérience desktop | 2/4 | **50 %** |
| **P1** | Signature Enterprise | 0/4 | **0 %** |
| **P2** | Observabilité / charge | 1/4 | **25 %** |
| **P2** | Sources et modules optionnels | 0/4 | **0 %** |

</details>

### 📚 Liens de référence

[🛡️ Sécurité et risques](SECURITY.md) · [🧭 Guide technique central](docs/PRAVDA-GUIDE.md) · [🧪 Recette #134](https://github.com/suvorovpravda/Pravda-Bot/issues/134) · [🔏 Signature #90](https://github.com/suvorovpravda/Pravda-Bot/issues/90) · [📦 GitHub Actions](https://github.com/suvorovpravda/Pravda-Bot/actions)

---

## 🇬🇧 English

Pravda-Bot is a Windows-native Rust/Tauri 2 market-monitoring and paper-simulation application. MSI/NSIS installers and independent Doctor/Harness diagnostics are built in CI. A consolidated one-click diagnostic EXE is being developed in [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138). Clean offline VM acceptance, genuine modular installation and Enterprise signing are **not yet certified**. Live trading remains **disabled**. See [Security](SECURITY.md).

## 🇷🇺 Русский

Pravda-Bot — локальное приложение для Windows на Rust и Tauri 2. Пакеты MSI/NSIS и диагностические инструменты собираются в CI. Единая тестовая программа находится в разработке. Полностью автономная установка на чистой виртуальной машине и цифровая подпись Enterprise ещё не подтверждены. **Реальная торговля отключена.** См. [безопасность](SECURITY.md).
