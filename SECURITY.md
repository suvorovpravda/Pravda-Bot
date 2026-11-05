<div align="center">

# 🛡️ PRAVDA-BOT / SECURITY
### Frontières de confiance · Protections · Risques · Publication Windows

![Mode](https://img.shields.io/badge/Architecture-Zero%20Trust-2563eb?style=for-the-badge)
![Secret](https://img.shields.io/badge/Secrets-Hors%20interface-16a34a?style=for-the-badge)
![Trading](https://img.shields.io/badge/Trading%20réel-OFF-7c3aed?style=for-the-badge)
![Release](https://img.shields.io/badge/Enterprise%20signé-NON%20VALIDÉ-dc2626?style=for-the-badge)

**[📌 État actuel](#-état-actuel-et-limites)** · **[🔐 Frontières](#-frontières-de-confiance)** · **[🧪 Contrôles](#-contrôles-effectivement-présents)** · **[⚠️ Risques](#-risques-résiduels-et-limites)** · **[📩 Signalement](#-signaler-une-vulnérabilité)**

**[⬅️ README](README.md)** · **[🧭 Guide central](docs/PRAVDA-GUIDE.md)** · **[📋 Recette P0 #134](https://github.com/suvorovpravda/Pravda-Bot/issues/134)**

</div>

---

## 🇫🇷 État actuel et limites

> **Aucune certification de sécurité indépendante n'est revendiquée.** Les validations CI et CodeQL sont des preuves de contrôles précis sur un commit, pas des garanties générales contre toutes les attaques. L'objectif de livraison est fixé au **30 novembre 2026**, sous réserve des critères de recette.

| Domaine | État | Constat |
| :--- | :---: | :--- |
| Fenêtre native / ACL Tauri | 🟢 En place | Permissions de la fenêtre principale explicitement définies |
| Authentification / révocation | 🟢 En place | Vérifications Rust, rejet des résultats de sessions périmées |
| Stockage / sauvegarde SQLite | 🟢 Tests intégrés | Les restaurations complexes et profils utilisateur demandent une qualification séparée |
| Doctor et Harness | 🟢 Intégrés | Diagnostic passif et simulation synthétique sans transaction |
| Centre de test unifié | 🟡 En revue | [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138), pas encore livré |
| MSI / NSIS sur runner Windows | 🟢 Vérifiés en CI | Ne prouvent pas le scénario sans WebView2 ni réseau |
| VM isolée sans WebView2 | 🟠 Non vérifiée | Critère bloquant [#134](https://github.com/suvorovpravda/Pravda-Bot/issues/134) |
| Installation réellement modulaire | 🟠 Non validée | Options Standard / Personnalisée non homologuées |
| Chaîne Authenticode | 🔴 Non attestée | Certificat, identité, signature, horodatage et réputation [#90](https://github.com/suvorovpravda/Pravda-Bot/issues/90) |
| Ordres de trading réels | 🔒 Interdits | Aucun ordre, retrait ou accès privé privilégié sans [#89](https://github.com/suvorovpravda/Pravda-Bot/issues/89) |

**Preuve de référence au 5 novembre 2026 :** [workflow Windows #41972245070](https://github.com/suvorovpravda/Pravda-Bot/actions/runs/41972245070) réussi sur le commit **b2fb618b53408b0fd5fd2c4265cdfe0a05ffc898**. Le dernier état de la branche principale doit toujours être revérifié après un nouveau commit.

### 🔐 Frontières de confiance

| Frontière | Exigence | Comportement interdit |
| :--- | :--- | :--- |
| Interface WebView2 → Rust | Permissions Tauri minimales, session valide, autorisation par commande | Contourner l'auth via JS, popup ou fenêtre non autorisée |
| Requête asynchrone → résultat | Lier la réponse à la session qui a démarré la requête | Réutiliser une réponse d'une session révoquée après reconnexion |
| Secret / coffre Windows | Lecture native contrôlée, jamais envoyé à la WebView | Copier clés, jetons ou mots de passe dans HTML, logs, JSON |
| Flux public / privé | Consentement explicite, destinations limitées et refus sécurisé | Activer les API privées à partir d'un score ou d'un simple indicateur UI |
| SQLite / WAL | Transactions cohérentes, ne jamais réparer aveuglément | Supprimer les données ou recréer le mot de passe pendant un test |
| Installateur Windows | Manifeste SHA-256, provenance et signature vérifiée si revendiquée | Déclarer un ZIP unsigned comme Enterprise ou supprimer SmartScreen |
| Harnais de production | Fixtures synthétiques, isolement réseau OS/VM pour essais air-gapped | Transmettre une opération boursière ou démarrer des ordres live |

![CSP](https://img.shields.io/badge/CSP-Scripts%20locaux-16a34a)
![IPC](https://img.shields.io/badge/IPC-Session%20Rust-2563eb)
![Données](https://img.shields.io/badge/SQLite-WAL-0e7490)
![Contournement](https://img.shields.io/badge/Bypass-INTERDIT-dc2626)

### ✅ Contrôles effectivement présents

Le dépôt possède des commandes IPC explicitement nommées, des tests de refus de fenêtres non autorisées, des contrôles Rust de session, une CSP sans scripts distants, un vérificateur d'authentification PBKDF2-SHA256, la persistance SQLite, et des contrôles de non-régression WebView2. Les résultats asynchrones refusés après déconnexion ne doivent pas effacer l'interface de la session suivante.

Le Doctor Windows est **passif** : il ne transmet aucune donnée par DNS, HTTP ou TCP et ne répare rien automatiquement. Il produit les états **PASS**, **WARN**, **FAIL**, **SKIP** ; **SKIP ne vaut jamais PASS**. Le Harness lance des scénarios synthétiques de réussite, refus, crash et timeout. Ni Doctor ni Harness ne constituent une certification de performance financière ou une recette d'installation sur machine vierge.

Le centre de test intégré en [PR #138](https://github.com/suvorovpravda/Pravda-Bot/pull/138) vise à regrouper ces contrôles dans un **seul fichier EXE** avec rapport HTML et JSON local. **Il n'est pas homologué tant que sa propre compilation et la recette Windows ne sont pas passées.** Aucun rapport n'est envoyé automatiquement ; les noms de contrôle sont filtrés pour limiter la divulgation.

### 🪟 Installation et provenance Windows

![WebView2](https://img.shields.io/badge/WebView2-Prérequis%20embarqué-16a34a)
![Offline](https://img.shields.io/badge/VM%20hors%20réseau-À%20ATTESTER-f59e0b)
![MSI](https://img.shields.io/badge/MSI%20%2B%20NSIS-Candidats%20de%20test-2563eb)

Le fichier de configuration Tauri utilise **offlineInstaller** avec installation silencieuse de WebView2. Les paquets construits contiennent MSI, NSIS, l'application Rust et les compagnons diagnostiques. Le [workflow Windows](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/windows-package.yml) contrôle les empreintes SHA-256 et le manifeste de provenance sur le commit de la compilation. Cela ne prouve pas l'installation dans une VM Windows initialement dépourvue de WebView2 et totalement déconnectée d'Internet.

La [recette #134](https://github.com/suvorovpravda/Pravda-Bot/issues/134) exige une VM jetable, isolation réseau réalisée **dans l'hyperviseur**, snapshot propre, installation et suppression MSI puis NSIS, redémarrage, données anonymisées, onboarding natif, conservation SQLite et vérification de l'absence de port HTTP pour l'interface. Il ne faut jamais désinstaller WebView2 sur le PC personnel pour reproduire ce scénario.

Le mode **Standard / Personnalisé** doit désactiver réellement les modules facultatifs dans Rust, pas seulement masquer un bouton. L'authentification, les permissions minimales, SQLite, la vérification du binaire et les protections financières restent obligatoires.

Une distribution **Enterprise signée** requiert la vérification Authenticode de chaque EXE, du MSI, du NSIS, de la chaîne de confiance et de l'horodatage, conformément à [#90](https://github.com/suvorovpravda/Pravda-Bot/issues/90). Aucun fichier non signé ne doit être présenté comme certifié.

### ⚠️ Risques résiduels et limites

| Gravité | Risque | Limite réelle / remédiation |
| :--- | :--- | :--- |
| 🔴 Critique | Trading ou retrait non autorisé | Trading réel désactivé ; autorisation et audit indépendants exigés |
| 🔴 Élevée | Exécutable remplacé / binaire sans provenance | SHA-256 exact, signature si revendiquée, aucune exception antivirus |
| 🟠 Élevée | Installation en VM sans réseau et WebView2 | **NON VÉRIFIÉE** ; test G1 obligatoire |
| 🟠 Élevée | Choix d'installation purement décoratif | **NON HOMOLOGUÉ** ; désactivation Rust exigée |
| 🟠 Modérée | Compromission locale du compte Windows | Le coffre natif ne protège pas de tous les malwares sous le même compte |
| 🟡 Modérée | Interruption WAL, corruption ou migration partielle | Ne pas réparer l'original ; diagnostiquer et restaurer une copie cohérente |
| 🟡 Modérée | API indisponible, prix périmé ou source inconnue | Afficher une indisponibilité réelle, jamais une cotation inventée |

**VPN / Proton :** une application VPN installée ou un abonnement actif ne prouve pas que le tunnel et son kill switch fonctionnent. Aucune vérification non effectuée ne doit apparaître comme « protégé ». L'utilisateur doit conserver la maîtrise des options du VPN officiel, sans transmettre d'identifiants au projet.

**Confidentialité :** ne pas partager les fichiers d'authentification, bases SQLite, fichiers WAL/SHM, clés API, PFX, tokens, logs contenant des identités ou exports de comptes. Les rapports de test destinés au dépannage doivent rester expurgés et être vérifiés avant tout partage.

<details>
<summary><strong>📊 Baseline des neuf indicateurs historiques P0–P2 (ouvrir)</strong></summary>

Ces fractions sont conservées avec le README et le guide central pour la continuité des jalons historiques. **Elles ne sont pas une mesure globale de sécurité.**

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

### 📩 Signaler une vulnérabilité

Privilégier le [signalement de sécurité GitHub](https://github.com/suvorovpravda/Pravda-Bot/security). Décrire le comportement reproductible, le commit concerné et les contrôles déjà réalisés. Ne jamais joindre de mot de passe, clé, base utilisateur ou log brut sensible. Une capture doit masquer toute donnée personnelle et tout jeton.

Pour un problème de démarrage ou de données locales, préférer le rapport expurgé du Doctor ; un avertissement ne prouve pas automatiquement une compromission. Une anomalie de sécurité ne doit pas être corrigée en désactivant l'antivirus, l'UAC, SmartScreen ou le contrôle de session.

---

## 🇬🇧 English

Pravda-Bot applies native Rust authorization, a restricted Tauri/WebView2 IPC boundary, local SQLite and Windows-native diagnostics. CI success is not a security certification. Offline-clean-VM installation, true modular installer behavior and Enterprise signatures remain unverified. **Live orders and withdrawals are disabled.** Security concerns should be reported via [GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security), without personal data or credentials.

## 🇷🇺 Русский

Pravda-Bot использует локальную авторизацию Rust, ограниченные IPC-разрешения Tauri и SQLite. Успешная сборка CI не означает независимую сертификацию безопасности. Полная проверка установки без сети и подпись Enterprise ещё не завершены. **Реальная торговля отключена.** Уязвимости следует сообщать через [GitHub Security](https://github.com/suvorovpravda/Pravda-Bot/security), без паролей и ключей.
