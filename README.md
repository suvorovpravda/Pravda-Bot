# Pravda-Bot

**Pravda-Bot** est un projet Python de surveillance de marchés crypto et de données énergétiques européennes. Il reçoit des cotations publiques Kraken et Binance, mesure leurs écarts, les enregistre pour analyse et fournit un tableau de bord web local.

> **Avertissement important :** le bot ne passe aucun ordre et ne nécessite aucune clé d’échange. Il lit des prix de dernier échange, pas des prix acheteur/vendeur exécutables. La simulation papier est indicative : elle ne modélise pas les carnets, les exécutions, le slippage ni toutes les contraintes d’une vraie stratégie. Ce projet ne fournit pas de conseil financier.

## Sommaire

- [Fonctionnalités](#fonctionnalités)
- [Ce que le projet ne fait pas](#ce-que-le-projet-ne-fait-pas)
- [Prérequis](#prérequis)
- [Installation](#installation)
- [Démarrage rapide](#démarrage-rapide)
- [Tableau de bord](#tableau-de-bord)
- [Configuration](#configuration)
- [Données et fichiers produits](#données-et-fichiers-produits)
- [Analyse hors ligne](#analyse-hors-ligne)
- [Tests et qualité](#tests-et-qualité)
- [Dépannage](#dépannage)
- [Sécurité et confidentialité](#sécurité-et-confidentialité)
- [Structure du dépôt](#structure-du-dépôt)
- [Contribuer](#contribuer)
- [Licence](#licence)

## Fonctionnalités

### Surveillance crypto

- Flux ticker publics WebSocket de **Kraken** et **Binance** pour BTC/USDT, ETH/USDT et SOL/USDT.
- Détection des flux déconnectés, trop anciens ou désynchronisés, avec reconnexion à délai exponentiel.
- Calcul d’un écart brut, estimation simplifiée des frais, seuils configurables et confirmation/hystérésis des alertes.
- Alertes de mouvement de prix et d’opportunité via ntfy, si un sujet privé est configuré.
- Analyse facultative des opportunités admissibles avec un serveur local compatible LM Studio.
- Simulateur papier en portefeuille ou à taille fixe ; aucune transaction réelle n’est envoyée.

### Tableau de bord local

- Cotations crypto reçues du bot et graphique des ticks observés depuis l’ouverture de la page.
- Prix crypto convertis et présentés en **EUR et CHF**, avec choix de devise pour le graphique. Les conversions utilisent les taux de référence quotidiens BCE diffusés par Frankfurter et l’hypothèse indicative USDT ≈ USD.
- État et ancienneté visibles par flux ; un prix périmé est signalé et ne doit pas être considéré comme une cotation actuelle.
- Observations de flux physiques transfrontaliers fournies par ENTSOG, actualités trouvées par GDELT avec repli sur BBC World RSS, et sources/dates affichées.
- Interface locale en français avec une palette bleu, blanc et rouge.
- Le serveur écoute sur `127.0.0.1` par défaut ; les autres appareils du réseau n’y ont pas accès par défaut.

## Ce que le projet ne fait pas

- **Pas d’ordres réels :** la surveillance des marchés et le simulateur papier ne sont pas un système de trading automatisé.
- **Pas de cours TTF :** aucun fournisseur de prix de marché autorisé n’est connecté. Le tableau de bord ne présente donc ni cours TTF ni unité de cotation comme s’ils étaient disponibles. Le lien ICE est une référence aux spécifications du contrat, pas un flux de prix.
- **Pas de carte de tracés réels de pipelines :** la visualisation ENTSOG représente des observations à des points transfrontaliers et leurs coordonnées publiées ; ce n’est pas un réseau routier ou un tracé de conduite.
- **Pas de suivi AIS des navires gaziers :** aucune position ou route commerciale de navire n’est récupérée. Il faut intégrer un fournisseur AIS et respecter ses clés, licences et droits de redistribution avant d’afficher ces données.
- **Pas de système opérationnel de renseignement géopolitique :** les titres de presse sont des informations rapportées, pas des incidents vérifiés, des évaluations de menace ou des prédictions.
- **Pas de Forex tick par tick :** les taux EUR/CHF et les autres devises fiat proviennent des taux de référence quotidiens BCE, pas d’un flux interbancaire en temps réel.
- **Pas de déploiement cloud ou d’application mobile :** ce dépôt est une application Python locale. Il ne contient pas l’architecture Node.js/PostgreSQL/PostGIS, Next.js ou Expo d’un autre projet.

## Feuille de route — idées pour les prochaines versions

Les pistes ci-dessous sont des **améliorations envisagées**, pas des fonctions déjà disponibles. Leur ordre pourra évoluer selon les sources de données accessibles, leurs licences et les besoins des utilisateurs.

### Priorité 1 — Un terminal plus utile au quotidien

- **Historique de prix persistant** : conserver les ticks et produire des graphiques multi-échelles (jour, semaine, mois), avec export CSV.
- **Prix crypto en EUR et CHF plus lisibles** : montrer l’heure et la source du taux de conversion, son âge et la devise de cotation d’origine ; proposer des alertes de seuil configurables dans la devise choisie.
- **État des sources sur une seule vue** : distinguer clairement « live », « retardé », « périmé » et « indisponible », avec heure du dernier événement pour chaque plateforme et fournisseur.
- **Résumé quotidien exportable** : mouvements de marché, événements énergétiques observés et liens vers les sources, sans transformer les articles en conclusions automatisées.

### Priorité 2 — Carte géographique et données énergie

- **Carte géographique réelle des infrastructures gazières** : afficher pipelines et terminaux à partir de données géographiques vérifiables, avec attribution, date, précision et droits de réutilisation. Les points ENTSOG actuels ne suffisent pas à reconstituer le tracé des conduites.
- **Routes commerciales des navires GNL** : visualiser positions et trajets uniquement après intégration d’un fournisseur AIS adapté, avec sa clé, ses conditions d’utilisation et ses droits de redistribution. Les positions ne doivent pas être extrapolées ni inventées.
- **Prix TTF vérifié** : intégrer un fournisseur autorisé de prix de marché, indiquer contrat/échéance, devise, unité, horodatage, statut retardé ou live et conditions de licence. En attendant, le tableau de bord continuera à afficher l’indisponibilité plutôt qu’un prix estimé présenté comme réel.
- **Contexte énergétique** : ajouter, lorsque les sources appropriées le permettent, niveaux de stockage, capacité et utilisation des terminaux GNL, incidents publiés et données météo pertinentes, chaque série avec son unité et son intervalle de publication.

### Priorité 3 — Veille géopolitique prudente

- **Carte mondiale d’événements** avec sources complémentaires (par exemple GDELT, ACLED et fournisseurs d’actualités), liens vers les publications originales, dates, zones géographiques et dédoublonnage traçable.
- **Indicateurs de contexte par pays** clairement expliqués : couverture des sources, période observée, méthode de calcul et incertitude. Un score de couverture médiatique ne doit pas être présenté comme une mesure certaine du risque réel.
- **Filtres et suivi personnalisé** par zone, sujet et période, avec notifications configurables et possibilité de corriger les erreurs de géolocalisation ou de catégorisation.
- **Journal de provenance** pour retrouver quelle source et quelle règle ont produit chaque élément affiché.

### À plus long terme — accès multi-appareils

- Étudier une API et une base de données adaptées si la conservation et le partage des historiques deviennent nécessaires.
- Envisager une interface mobile ou une version web hébergée seulement après définition de l’authentification, de la protection des clés et des coûts d’exploitation.
- Garder l’application locale simple tant que ces besoins ne justifient pas une migration vers une architecture plus lourde.

**Dépendances importantes :** plusieurs pistes nécessitent des comptes fournisseurs, des licences payantes ou des droits explicites de stockage et de redistribution. Aucune donnée d’actualité, AIS ou de marché sous licence ne sera simulée pour donner l’illusion d’un flux réel.

## Prérequis

- Windows 10/11 (le lanceur un clic est destiné à Windows) ou un environnement Python compatible.
- Python **3.12** recommandé (le lanceur Windows recherche Python 3.12 ; le code utilise les fonctionnalités Python 3.10+).
- Accès Internet pour les flux publics des plateformes et les données ENTSOG, GDELT/BBC et Frankfurter.
- LM Studio est facultatif pour l’analyse LLM. Le bot peut tourner sans modèle disponible tant qu’aucune analyse ne doit être demandée.
- Google Chrome est facultatif ; le lanceur ouvre le navigateur par défaut si Chrome n’est pas trouvé.

## Installation

Dans PowerShell :

```powershell
git clone https://github.com/suvorovpravda/Pravda-Bot.git
Set-Location Pravda-Bot
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
Copy-Item .env.example .env
```

Le fichier `.env` local est ignoré par Git. Ouvrez-le pour régler les paramètres nécessaires. Les valeurs d’exemple ne sont pas des secrets et le sujet ntfy fourni dans `.env.example` est un placeholder : remplacez-le ou laissez les notifications désactivées.

## Démarrage rapide

### Windows : lancement en un clic

Double-cliquez sur `Lancer_Pravda_Bot.bat` à la racine du dépôt. Le script :

1. démarre le bot et le serveur du tableau de bord dans deux fenêtres ;
2. ouvre `http://127.0.0.1:8080` dans Chrome ou dans le navigateur par défaut.

Gardez les fenêtres du bot et du tableau de bord ouvertes. Fermer le bot arrête les flux d’échange ; fermer le serveur interrompt la page. Vous pouvez enregistrer l’adresse locale comme favori Chrome, mais ce favori ne fonctionne que lorsque les deux processus tournent sur le même ordinateur.

### Démarrage manuel

Dans deux fenêtres PowerShell distinctes, depuis le dépôt :

```powershell
# Fenêtre 1 : bot et flux de marché
.\.venv\Scripts\Activate.ps1
python main.py
```

```powershell
# Fenêtre 2 : tableau de bord web local
.\.venv\Scripts\Activate.ps1
python dashboard.py
```

Ouvrez ensuite [http://127.0.0.1:8080](http://127.0.0.1:8080). Sans le bot, les prix crypto ne sont pas alimentés. Le tableau de bord peut quand même afficher les données énergétiques et les devises disponibles.

### Lancer uniquement le bot ou le tableau de bord

```powershell
python main.py
python dashboard.py
```

Le tableau de bord accepte aussi des chemins et paramètres personnalisés :

```powershell
python dashboard.py --csv .\spread_observations.csv --lng-csv .\lng_observations.csv --port 8081 --threshold 0.05 --stale-after 30
```

L’interface locale restera liée à `127.0.0.1`, y compris avec un port différent.

## Tableau de bord

### Marchés crypto et devises

Le bot écrit ses derniers ticks dans `market_snapshot.json` par remplacement atomique. Le tableau de bord lit ce fichier local chaque seconde. Une cotation devient périmée après **10 secondes** dans ce snapshot ; le tableau de bord affiche alors le dernier prix connu avec un indicateur « périmé » plutôt que de le faire passer pour un prix récent.

La conversion de prix crypto suit cette approximation :

```text
prix EUR indicatif = prix USDT / (USD pour 1 EUR)
prix CHF indicatif = prix EUR × (CHF pour 1 EUR)
```

Les cours crypto restent des prix de dernier échange libellés à l’origine en USDT ; les montants EUR et CHF sont des conversions indicatives, pas des prix d’exécution. Le taux de change utilisé est une référence BCE quotidienne. La table, le graphique et le journal secondaire du tableau de bord affichent les conversions dans ces deux devises lorsque les taux nécessaires sont disponibles.

### Énergie, actualités et TTF

- Les observations physiques proviennent de la plateforme de transparence **ENTSOG**. Elles conservent les unités et périodes de la source et ne sont ni dédoublonnées ni équivalentes à des importations/exportations nettes nationales.
- La carte schématique positionne les points ENTSOG disponibles. Elle ne prétend pas représenter les routes réelles des gazoducs.
- Les actualités proviennent de **GDELT** ; le flux BBC World RSS sert de repli quand la source GDELT est indisponible. Les articles restent à vérifier auprès de leurs éditeurs.
- Les cours de référence fiat sont fournis par l’API Frankfurter à partir des taux BCE. Ils sont actualisés selon la disponibilité de la source et ne sont pas des prix Forex en continu.
- Aucun prix TTF n’est affiché sans flux de marché vérifiable et droit d’usage approprié. Les spécifications ICE ne remplacent pas un fournisseur de cours.

Les données ENTSOG sont mises en cache pendant cinq minutes, les actualités pendant quinze minutes, les points de connexion ENTSOG pendant vingt-quatre heures et les taux de change pendant six heures côté serveur. La publication des sources amont peut être plus lente que ces intervalles.

### Routes HTTP locales

| Route | Contenu |
| --- | --- |
| `/` | Interface du tableau de bord |
| `/api/market` | Snapshot local des prix crypto et état de fraîcheur |
| `/api/fx` | Taux de référence BCE disponibles |
| `/api/eu/flows` | Observations de flux physiques ENTSOG |
| `/api/geopolitics` | Articles GDELT ou BBC World RSS |
| `/api/gas-price` | État de disponibilité du prix TTF ; aucune cotation simulée |
| `/api/metrics` | Métriques du processus du tableau de bord, pas un contrôle de santé du bot |
| `/data` | Observations du journal des spreads |
| `/lng-data` | Échantillons du fichier local GNL, s’il existe |

Ces routes ne constituent pas une API publique ou authentifiée : le serveur est prévu pour un usage local.

## Configuration

Les paramètres peuvent être définis dans `.env` ou dans l’environnement du processus. Les variables d’environnement existantes ont priorité sur les valeurs chargées depuis `.env`.

### Paramètres du bot

| Variable | Valeur par défaut | Rôle |
| --- | --- | --- |
| `LM_STUDIO_API_BASE` | `http://192.168.1.146:1234/v1` | URL de l’API locale compatible LM Studio. Remplacez-la par `http://127.0.0.1:1234/v1` si LM Studio tourne sur ce PC. |
| `LM_MODEL_NAME` | `qwen/qwen3.5-9b` | Identifiant du modèle servi par LM Studio. |
| `TRADING_MODE` | `paper` | Libellé d’affichage ; ne change pas le comportement d’exécution et n’active pas d’ordres réels. |
| `MIN_SPREAD_THRESHOLD` | `0.05` | Seuil minimal de spread brut, en pourcentage. |
| `MAX_ALLOWED_SPREAD_PERCENT` | `5.0` | Spread maximal accepté avant rejet comme anomalie, en pourcentage. |
| `SPREAD_LOG_INTERVAL_SECONDS` | `2` | Intervalle d’échantillonnage des spreads par paire. |
| `DASHBOARD_INTERVAL_SECONDS` | `30` | Intervalle d’actualisation du tableau Rich dans le terminal. |
| `MARKET_DATA_STALE_SECONDS` | `4` | Âge maximal d’un flux de marché pour évaluer un spread. |
| `MARKET_DATA_MAX_SKEW_SECONDS` | `3` | Désynchronisation maximale entre les deux dernières réceptions. |
| `ALERT_CONFIRMATION_SECONDS` | `3` | Durée pendant laquelle le signal doit persister avant déclenchement. |
| `ALERT_REARM_HYSTERESIS_PERCENT` | `0.05` | Baisse minimale sous le seuil avant de réarmer un signal. |
| `PRICE_ALERT_COOLDOWN_SECONDS` | `5` | Délai anti-rafale par paire pour les alertes de mouvement ; `0` désactive seulement ce délai par paire. |
| `NTFY_GLOBAL_COOLDOWN_SECONDS` | `10` | Intervalle global minimal entre envois ntfy. |
| `NTFY_TOPIC_URL` | vide | URL HTTPS d’un sujet ntfy privé ; vide = notifications désactivées. |

### Paramètres de la simulation papier

| Variable | Valeur par défaut | Rôle |
| --- | --- | --- |
| `PAPER_MODE` | `portfolio` | `portfolio` utilise une fraction du solde ; `fixed` utilise une taille fixe. |
| `PAPER_STARTING_BALANCE_USDT` | `1000` | Solde virtuel initial. |
| `PAPER_POSITION_PERCENT` | `10` | Part du cash disponible allouée en mode `portfolio`, entre 0 et 100. |
| `PAPER_FIXED_POSITION_USDT` | `100` | Taille maximale de position en mode `fixed`, limitée au cash disponible. |
| `PAPER_EXIT_SPREAD_PERCENT` | `0.05` | Seuil de clôture simulée selon le sens du trade. |
| `PAPER_MAX_HOLD_SECONDS` | `3600` | Durée maximale de détention simulée avant clôture. |

Le simulateur travaille en USDT en interne. Les montants d’état du portefeuille et les CSV du simulateur ne sont pas convertis en EUR/CHF par le tableau de bord.

### Paramètres du tableau de bord

| Variable ou argument | Valeur par défaut | Rôle |
| --- | --- | --- |
| `DASHBOARD_PORT` ou `--port` | `8080` | Port HTTP local. |
| `SPREAD_LOG_FILE` ou `--csv` | `spread_observations.csv` | Journal d’observations du bot. |
| `LNG_LOG_FILE` ou `--lng-csv` | `lng_observations.csv` | Fichier d’observations GNL locales. |
| `DASHBOARD_STALE_AFTER_SECONDS` ou `--stale-after` | `30` | Âge auquel le journal de spreads est qualifié d’obsolète dans le dashboard. |
| `MIN_SPREAD_THRESHOLD` ou `--threshold` | `0.05` | Seuil indiqué dans le dashboard pour le journal. |
| `MARKET_SNAPSHOT_FILE` | `market_snapshot.json` | Chemin du snapshot local des cotations crypto. |

Le bot et le serveur ont deux seuils de fraîcheur distincts : `MARKET_DATA_STALE_SECONDS` sert à protéger l’évaluation des opportunités ; le snapshot web déclare les prix périmés après 10 secondes ; `DASHBOARD_STALE_AFTER_SECONDS` concerne le journal CSV.

## Données et fichiers produits

| Fichier | Usage | Versionné dans Git |
| --- | --- | --- |
| `arbitrage_log.csv` | Opportunités et analyses LLM retenues. | Non |
| `spread_observations.csv` | Échantillons numériques de spreads, seuils, état et horodatages. | Non |
| `paper_trades.csv` | Transactions virtuelles clôturées et résultats estimés. | Non |
| `market_snapshot.json` | Derniers prix reçus des flux d’échange pour le dashboard local. | Non |
| `lng_observations.csv` | Exemples locaux produits par l’outil GNL autonome. | Non |
| `.env` | Configuration et éventuelles URL privées. | Non |

Ces fichiers sont ignorés par `.gitignore`. Ne forcez pas leur ajout au dépôt s’ils contiennent des URL privées ou des données locales.

### Spread, frais et interprétation

Le taux de frais estimé est fixe à **0,36 %** et sert uniquement à un calcul indicatif du spread net. Il ne prend pas en compte le slippage, les frais réseau/retrait, les délais ou les tarifs particuliers des plateformes.

Le détecteur calcule le spread brut absolu comme `abs(Kraken - Binance) / min(Kraken, Binance)`. Le simulateur papier utilise un spread signé différent, `(Kraken - Binance) / Binance`. Leurs pourcentages ne sont donc pas directement comparables. Les observations historiques aident à diagnostiquer les données et le comportement du bot ; elles ne démontrent pas qu’une opération serait exécutable ou rentable.

## Analyse hors ligne

Les scripts d’analyse ne se connectent pas aux plateformes :

```powershell
python analyze_spreads.py
python analyze_spreads.py .\spread_observations.csv --output .\spread_summary.csv
python analyze_paper_results.py
```

`analyze_spreads.py` résume par paire les échantillons valides, moyenne, médiane, percentiles, maximum, seuils atteints, spreads nets positifs et opportunités déclenchées. Les statuts de marché exclus ou non vérifiés sont présentés séparément.

`analyze_paper_results.py` compte les observations valides, les opportunités déclenchées dans les observations valides, les anomalies et les autres statuts. Une opportunité rejetée ou une observation obsolète ne doit pas être interprétée comme une simulation exécutée.

### Échantillons GNL (outil de démonstration)

```powershell
python lng_observations.py
python lng_observations.py --csv .\data\lng_samples.csv
```

Ce script écrit des valeurs d’exemple **simulées** TTF, JKM et Henry Hub dans un CSV local. Il ne récupère ni cours courants, ni données d’expédition, ni positions de navires. Ne confondez pas ce fichier d’échantillons avec une cotation de marché.

## Tests et qualité

Avec Python 3.12 et les dépendances installées :

```powershell
python -m pytest -q
python -m compileall -q .
git diff --check
```

La CI GitHub Actions exécute aussi la compilation/importation et la suite de tests lors des pushes et des pull requests vers `main`.

## Dépannage

### La page affiche « ERR_CONNECTION_REFUSED »

Le serveur n’est pas lancé ou s’est arrêté. Depuis le dossier du dépôt, démarrez `python dashboard.py` et gardez la fenêtre ouverte, puis rechargez `http://127.0.0.1:8080`. Vérifiez aussi qu’aucun autre programme n’utilise le port 8080 ; au besoin, lancez le serveur avec `--port 8081`.

### Les prix crypto sont absents ou périmés

Vérifiez que `main.py` est en cours d’exécution, que la connexion Internet et les WebSockets ne sont pas bloqués, et consultez l’état affiché séparément pour Kraken et Binance. Un prix périmé est intentionnellement conservé comme dernier cours connu avec un avertissement ; il ne représente pas un cours actuel.

### Les taux EUR/CHF ne sont pas disponibles

Les conversions nécessitent les valeurs USD et CHF des taux BCE via Frankfurter. La source est quotidienne et peut être indisponible ou ne pas avoir publié de nouvelle référence les jours non ouvrés. Le dashboard signale l’indisponibilité au lieu de fabriquer un taux.

### Les flux ENTSOG ou les actualités échouent

Ces sources publiques sont externes, peuvent être retardées, limitées ou momentanément inaccessibles. Le serveur met les données en cache et réessaie lors de l’actualisation suivante. Les actualités ont un repli BBC ; les observations ENTSOG n’ont pas de substitut fabriqué.

### Le modèle LM Studio ne répond pas

Vérifiez que le serveur LM Studio est lancé et que `LM_STUDIO_API_BASE` et `LM_MODEL_NAME` correspondent à sa configuration. Utilisez `127.0.0.1` si LM Studio et le bot tournent sur le même ordinateur. L’analyse est facultative pour le suivi des prix ; une panne du modèle ne transforme pas les données de marché.

### Le lanceur indique que Python ou les dépendances manquent

Installez Python 3.12 ou créez `.venv`, puis exécutez :

```powershell
python -m pip install -r requirements.txt
```

Relancez ensuite `Lancer_Pravda_Bot.bat`.

## Sécurité et confidentialité

- Aucun secret ou identifiant d’API d’échange n’est requis pour les flux publics.
- Ne publiez jamais `.env`, une clé, un mot de passe ou une URL de sujet ntfy.
- Les sujets ntfy ne sont pas des espaces privés authentifiés : toute personne qui connaît l’URL peut potentiellement lire ou publier des messages. Utilisez un sujet aléatoire, gardez-le confidentiel et faites-le tourner s’il est exposé.
- Le serveur web se lie à localhost par défaut et ne met en place ni authentification ni contrôle d’accès pour une exposition publique. Ne le publiez pas sur Internet sans une architecture sécurisée adaptée.
- LM Studio reçoit les symboles et prix inclus dans les demandes d’analyse. Pour garder ces informations sur votre machine, utilisez un serveur local de confiance et ne l’exposez pas à un réseau non fiable.
- Examinez `git status` et le diff avant de publier ; les journaux peuvent révéler vos habitudes d’utilisation.

## Structure du dépôt

```text
Pravda-Bot/
├── main.py                  # Flux d'échange, détection et bot
├── dashboard.py             # Serveur HTTP local et routes JSON
├── dashboard.html           # Interface du tableau de bord
├── live_market_data.py      # Snapshot crypto et taux de référence FX
├── energy_data.py           # Adaptateurs ENTSOG, GDELT/BBC et état TTF
├── bot_config.py            # Chargement et validation de configuration
├── market_data_health.py    # Fraîcheur et désynchronisation des flux
├── alert_gate.py            # Confirmation et hystérésis des alertes
├── reconnect.py             # Reconnexion WebSocket avec backoff
├── paper_simulator.py       # Simulation de portefeuille sans ordres
├── security_and_fees.py     # Contrôles de prix et calculs de spread
├── csv_schema.py            # Schéma des observations
├── bot_dashboard.py         # Résumé Rich du terminal
├── analyze_spreads.py       # Analyse historique des spreads
├── analyze_paper_results.py # Bilan des statuts et simulations papier
├── lng_observations.py      # Générateur d'échantillons GNL simulés
├── tests/                   # Tests unitaires
├── .env.example             # Exemple sans secret
├── requirements.txt         # Dépendances Python
└── Lancer_Pravda_Bot.bat    # Lanceur Windows en un clic
```

## Contribuer

Les contributions sont bienvenues. Pour proposer un changement :

1. créez une branche à partir de `main` ;
2. gardez les changements ciblés et ajoutez des tests pour tout comportement modifié ;
3. lancez les tests, la compilation et `git diff --check` ;
4. vérifiez que le diff ne contient ni secret ni journal local ;
5. ouvrez une pull request en décrivant le problème, la solution, les limites et les résultats des tests.

Pour toute intégration de prix TTF ou de suivi maritime, documentez le fournisseur, la fraîcheur, les limites, les licences et les droits de redistribution. Ne publiez pas de données simulées comme des observations live.

## Licence

Aucun fichier de licence n’est actuellement fourni dans ce dépôt. En attendant qu’une licence soit ajoutée, ne présumez pas que le code peut être redistribué ou réutilisé selon les conditions d’une licence open source.
