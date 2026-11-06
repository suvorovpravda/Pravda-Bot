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
| **Pravda-Setup.exe — un seul installateur utilisateur** | 🟠 En validation | [PR #145](https://github.com/suvorovpravda/Pravda-Bot/pull/145) ; embarque MSI et WebView2 offline, GUI physique non homologuée |
| **Tests pré/postproduction dans l'application** | 🟠 En validation | Commandes Rust IPC authentifiées depuis Paramètres ; VM, signature et restauration restent non vérifiées |
| Installation totalement hors réseau sans WebView2 initial | 🟠 Non prouvée | Recette VM indépendante requise |
| Installation Standard / Personnalisée | 🟠 À valider | Retrait des modules facultatifs côté Rust |
| Enterprise signée / horodatée | 🔴 Non livrée | [Issue #90](https://github.com/suvorovpravda/Pravda-Bot/issues/90) |
| Trading réel / retraits | 🔒 Désactivés | [Issue #89](https://github.com/suvorovpravda/Pravda-Bot/issues/89) |

![P0](https://img.shields.io/badge/P0-Recette%20Windows%20en%20cours-f59e0b)
![Plan](https://img.shields.io/badge/Plan%20initial%20%23106-Clôturé%20administrativement-64748b)
![GO](https://img.shields.io/badge/Production%20complète-NO--GO-dc2626)

**Point de vérité actuel : [issue #134 — recette Windows finale](https://github.com/suvorovpravda/Pravda-Bot/issues/134).** L'ancienne issue #106 est clôturée, mais cette clôture n'atteste pas les scénarios P0 restants.

### ⬇️ Téléchargements Windows

**Preuve du centre unifié (5 novembre 2026) :** [packaging Windows #42016900394](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/42016900394), sur le commit de PR **038c9318ca80ba3e6f67ccdc0fe237415f77ec21**, vérifié sur un second runner Windows (13/13 contrôles réussis). Cette PR a été fusionnée sur `main` par le commit **8b87783232b2c118d4e8d1e34c04b2f89dd6de8a**. Ne pas confondre la réussite de la CI de la PR avec une certification ou avec une validation de chaque commit ultérieur sur `main`.

| Archive GitHub Actions | Contenu | Utilisation |
| :--- | :--- | :--- |
| **pravda-windows-unsigned-candidate-installers** | MSI, NSIS, EXE, sommes SHA-256 | Installation Windows de test |
| **pravda-windows-troubleshooting** | Doctor Windows | Double-clic, diagnostic passif |
| **pravda-windows-production-test** | Harness Rust | Scénarios de simulation synthétiques |
| **pravda-windows-one-click-test-center** | Doctor + Harness + démarrage temporaire du Desktop | **Outil de CI / diagnostic isolé uniquement**, pas un installateur ni une session interactive |
| **pravda-windows-single-file-setup** | `pravda-setup.exe` — MSI hors ligne intégré | **Livrable utilisateur cible**, en construction et validation dans [PR #145](https://github.com/suvorovpravda/Pravda-Bot/pull/145) |

Pour récupérer une version, ouvrir [Windows installer packaging](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml), choisir un run **réussi sur le SHA souhaité**, descendre à **Artifacts**, puis extraire l'archive.

Les paquets MSI/NSIS configurent le prérequis officiel WebView2 avec **offlineInstaller** et installation silencieuse. Aucune installation manuelle de Python, Rust, Node ou MSVC n'est requise pour utiliser les paquets. L'essai sur VM Windows réellement sans WebView2 et sans Internet reste obligatoire avant de déclarer cette autonomie homologuée.

**Important :** une archive de test non signée peut déclencher un avertissement SmartScreen. Ne jamais désactiver SmartScreen, l'antivirus, le pare-feu ou l'UAC pour contourner une protection. Les archives GitHub Actions sont temporaires et ne constituent pas une Release Enterprise.

### 🧰 Centre de test unifié

![Rapport](https://img.shields.io/badge/Rapport-HTML%20coloré%20%2B%20JSON-7c3aed)
![Confidentialité](https://img.shields.io/badge/Diagnostic-Local-16a34a)
![Exécution](https://img.shields.io/badge/Utilisateur-Un%20double--clic-2563eb)

**Parcours utilisateur cible :** ouvrir `pravda-setup.exe` une seule fois, installer Pravda-Bot, puis démarrer le **vrai bureau Tauri**, sans limite artificielle de six secondes. Après authentification, ouvrir **Paramètres → Sécurité & dépannage** pour lancer les tests Rust **préproduction** ou **postproduction**, et copier leur rapport JSON expurgé. L'outil indépendant `pravda-test-center.exe` reste réservé aux contrôles de CI ; son ouverture temporaire du bureau ne permet pas une connexion interactive et ne doit pas être proposé comme installateur.

**Ne pas confondre :** les résultats locaux du centre intégré ne qualifient ni les installations sur deux VM sans réseau ni la signature Authenticode ni les mises à niveau. [PR #145](https://github.com/suvorovpravda/Pravda-Bot/pull/145) et [#134](https://github.com/suvorovpravda/Pravda-Bot/issues/134) restent soumis aux preuves au SHA final.

**Statut actuel : candidat de test non signé, validé sur runner Windows et par Codex (13/13 contrôles).** Ce n'est **pas** une publication Enterprise. Les tests qui exigent une VM réellement hors réseau, un installateur personnalisé ou une signature Enterprise restent **NON VÉRIFIÉS**, et le rapport conserve `SKIP` pour ces scénarios.

Le programme n'envoie pas le rapport vers un service externe. L'utilisateur doit examiner le JSON avant tout partage ; ne jamais transmettre mot de passe, clé privée, base SQLite ou historique financier.

### 🔁 Agent de diagnostic continu (PR #156 — non livré tant que la CI n'est pas validée)

Le binaire indépendant `pravda-test-center.exe` intègre Doctor, Harness et un bureau Tauri isolé. **Un double-clic démarre l'agent en arrière-plan**, puis il effectue un premier cycle et le répète toutes les 15 minutes. **Un second double-clic demande l'arrêt.** Il fonctionne dans la session Windows courante, sans service, tâche planifiée ou démarrage automatique à la connexion.

Le rapport à copier-coller dans ChatGPT s'appelle `PRAVDA-COPIER-ICI.txt` et se trouve sous `%LOCALAPPDATA%\Pravda-Bot\Diagnostic-Continu` avec les versions JSON/HTML et `PRAVDA-ETAT.txt`. Aucune transmission automatique n'est effectuée. Les contrôles sont hors réseau et utilisent des profils et une base de simulation jetables : **aucun ordre financier, aucune clé API, aucune réparation**. Les véritables parcours interactifs, l'installation sur VM vierge et l'updater signé restent **SKIP**.

Sur le runner Windows uniquement, `pravda-test-center.exe --monitor-once` effectue un cycle borné puis vérifie les fichiers de sortie; `--ci` conserve le test intégré historique. Pour l'utilisateur, il n'y a aucune commande à saisir. **Ce mode est en développement : télécharger uniquement un build dont les contrôles de la PR #156 ont réussi.**

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

Pour un **utilisateur final**, le livrable visé est **un seul `pravda-setup.exe`** (PR #145), qui installe le bureau Windows via son MSI intégré et le prérequis WebView2 hors ligne. L'application reste ensuite ouverte normalement et contient les tests pré/postproduction dans les paramètres. Pendant la validation, les anciens MSI/NSIS constituent des candidats techniques, non une livraison Enterprise homologuée. Il n'est pas nécessaire d'installer Rust, Python ou Node sur le poste utilisateur.

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

Pravda-Bot is a Windows-native Rust/Tauri 2 market-monitoring and paper-simulation application. MSI/NSIS installers and independent Doctor/Harness diagnostics are built in CI. A consolidated one-click diagnostic EXE was merged in [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138) and independently tested on a Windows CI runner; user-machine double-click acceptance is still pending. Clean offline VM acceptance, genuine modular installation and Enterprise signing are **not yet certified**. Live trading remains **disabled**. See [Security](SECURITY.md).

## 🇷🇺 Русский

Pravda-Bot — локальное приложение для Windows на Rust и Tauri 2. Пакеты MSI/NSIS и диагностические инструменты собираются в CI. Единая тестовая программа собрана и проверена в Windows CI; запуск двойным щелчком на компьютере пользователя ещё предстоит подтвердить. Полностью автономная установка на чистой виртуальной машине и цифровая подпись Enterprise ещё не подтверждены. **Реальная торговля отключена.** См. [безопасность](SECURITY.md).
