# Pravda-Bot

**Pravda-Bot** est un projet Python de surveillance de marchés crypto et de données énergétiques européennes. Il reçoit des cotations publiques Kraken et Binance, mesure leurs écarts, les enregistre pour analyse et fournit un tableau de bord web local.

**Langues : Français · [Русский](#russian) · [English](#english)**

> **Avertissement important :** le bot ne passe aucun ordre et ne nécessite aucune clé d’échange. Les signaux utilisent les prix de dernier échange. Lors d’un signal confirmé, une estimation séparée consulte les carnets publics et estime la profondeur, le prix moyen, les frais taker configurés, le slippage et les minima de marché. Cette estimation ne vérifie pas les soldes, l’inventaire, la marge, les emprunts, les frais réels du compte, les délais d’exécution ou les transferts ; elle ne constitue donc pas une preuve d’exécutabilité. Ce projet ne fournit pas de conseil financier.

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
- Estimation au signal confirmé d’un achat et d’une vente au marché sur les carnets publics Kraken/Binance : balayage de profondeur, VWAP, remplissage partiel, frais taker, slippage et limites/precision de quantité fournies par CCXT. Une estimation rejetée, partielle ou indisponible n’ouvre pas de nouvelle position papier.

### Tableau de bord local

- Cotations crypto reçues du bot et graphique des ticks observés depuis l’ouverture de la page.
- Prix crypto convertis et présentés en **EUR et CHF**, avec choix de devise pour le graphique. Les conversions utilisent les taux de référence quotidiens BCE diffusés par Frankfurter et l’hypothèse indicative USDT ≈ USD.
- Sélecteur d’interface **Français / Русский / English**, mémorisé localement dans le navigateur.
- État et ancienneté visibles par flux ; un prix périmé est signalé et ne doit pas être considéré comme une cotation actuelle.
- Observations de flux physiques transfrontaliers fournies par ENTSOG, actualités trouvées par GDELT avec repli sur BBC World RSS, et sources/dates affichées.
- Le serveur écoute sur `127.0.0.1` par défaut ; les autres appareils du réseau n’y ont pas accès par défaut.

## Ce que le projet ne fait pas

- **Pas d’ordres réels :** la surveillance des marchés et le simulateur papier ne sont pas un système de trading automatisé.
- **Pas de validation de compte :** les carnets publics ne révèlent ni soldes ni inventaire spot. L’estimation d’exécution indique explicitement que ces éléments ne sont pas vérifiés ; elle ne modélise pas non plus marge, emprunt, transferts, limites de risque propres au compte ou latence de placement.
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
| `PAPER_TAKER_FEE_PERCENT` | `0.1` | Frais taker estimés **par jambe** de l’estimation d’exécution ; remplacez-les selon vos tarifs, car le bot ne lit pas les frais privés du compte. |
| `PAPER_MAX_SLIPPAGE_PERCENT` | `0.5` | Slippage cumulé maximal estimé entre les deux jambes. |
| `PAPER_ORDERBOOK_MAX_AGE_SECONDS` | `2` | Âge local maximal autorisé des snapshots de carnet récupérés via REST. |

Le simulateur travaille en USDT en interne. Les carnets sont des instantanés publics REST avec heure de réception locale, pas un flux L2 continu. `estimate_only` signifie que profondeur, minima et contrôles configurés passent, pas que les soldes sont suffisants ; une estimation partielle n’ouvre pas de position papier. Cette vérification porte sur l’entrée du simulateur ; les positions et leur clôture dans `paper_trades.csv` utilisent encore le spread ticker, pas des remplissages réels de carnets. L’estimation n’intègre ni frais privés exacts, ni frais réseau/retrait, ni mouvements de portefeuille. Les montants du simulateur ne sont pas convertis en EUR/CHF par le tableau de bord.

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
| `paper_execution_estimates.csv` | Résultats détaillés des estimations d’exécution et rejets de carnets. | Non |
| `market_snapshot.json` | Derniers prix reçus des flux d’échange pour le dashboard local. | Non |
| `lng_observations.csv` | Exemples locaux produits par l’outil GNL autonome. | Non |
| `.env` | Configuration et éventuelles URL privées. | Non |

Ces fichiers sont ignorés par `.gitignore`. Ne forcez pas leur ajout au dépôt s’ils contiennent des URL privées ou des données locales.

### Spread, frais et interprétation

Le détecteur de signal conserve un taux simplifié fixe à **0,36 %** pour son estimation de spread net. Le contrôle d’exécution utilise séparément `PAPER_TAKER_FEE_PERCENT` pour chacune des deux jambes, les niveaux visibles du carnet, les limites de slippage et les minima de marché publics. Le taux de frais réel du compte n’est pas récupéré. Les frais réseau/retrait, les délais, les soldes, les emprunts et les éventuels changements de carnet pendant l’envoi ne sont pas simulés.

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
├── orderbook_execution.py  # Estimation publique des remplissages et de la profondeur
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

<a id="russian"></a>

# Русский

**Pravda-Bot** — локальный Python-проект для мониторинга крипторынка и европейской энергетики. Он получает публичные котировки Kraken и Binance, измеряет спреды, сохраняет наблюдения для анализа и предоставляет локальную веб-панель.

> **Важно:** бот не размещает реальные ордера и не требует ключей бирж. Сигналы используют цены последних сделок. При подтверждении сигнала отдельная оценка запрашивает публичные стаканы и рассчитывает глубину, среднюю цену, заданную комиссию taker, проскальзывание и минимальные параметры рынка. Она не проверяет балансы, спотовый инвентарь, маржу, займы, фактические комиссии аккаунта, задержки исполнения или переводы и не доказывает исполнимость сделки. Это не финансовая рекомендация.

## Возможности

### Мониторинг криптовалют

- Публичные WebSocket-такеры **Kraken** и **Binance** для BTC/USDT, ETH/USDT и SOL/USDT.
- Контроль отключения, устаревания и рассинхронизации потоков; повторное подключение с экспоненциальной задержкой.
- Расчёт спреда, приблизительной комиссии, настраиваемых порогов и подтверждение/гистерезис сигналов.
- Уведомления о движении цены и возможностях через ntfy при настройке темы.
- Необязательный анализ прошедших проверки сигналов через локальный сервер, совместимый с LM Studio.
- Бумажная симуляция портфеля или фиксированного размера; реальные сделки не отправляются.
- При подтверждённом сигнале оцениваются рыночные заявки по публичным стаканам Kraken/Binance: глубина, VWAP, частичное исполнение, комиссия taker, проскальзывание, лимиты количества и точность из CCXT. Отклонённая, частичная или недоступная оценка не открывает новую бумажную позицию.

### Локальная панель

- Котировки, полученные ботом, и график тиков с момента открытия страницы.
- Цены криптовалют в **EUR и CHF**: пересчёт использует ежедневные справочные курсы ЕЦБ через Frankfurter и приближение USDT ≈ USD.
- Статус и возраст каждого биржевого потока; устаревшая цена отмечается и не выдаётся за текущую.
- Наблюдения за физическими трансграничными потоками газа ENTSOG и новости GDELT с резервным RSS BBC World.
- Локальный интерфейс на французском, русском и английском языках; выбор запоминается браузером. Палитра сине-бело-красная.
- По умолчанию сервер доступен только на `127.0.0.1`.

## Ограничения и данные, которых пока нет

- **Реальные ордера не размещаются.** Мониторинг и бумажная симуляция не являются торговым роботом.
- **Счёт не проверяется.** Публичные стаканы не показывают балансы и спотовые активы. Оценка явно отмечает их как непроверенные; она также не моделирует маржу, займы, переводы, лимиты аккаунта или задержку отправки ордера.
- **Котировки TTF нет.** Не подключён лицензированный поставщик рыночных цен. Панель не показывает цену или единицу котировки TTF. Ссылка ICE описывает спецификации контракта, а не предоставляет поток котировок.
- **Нет реальной карты трубопроводов.** Схема ENTSOG показывает точки трансграничных наблюдений, а не трассы газопроводов.
- **Нет отслеживания газовозов по AIS.** Для отображения позиций и маршрутов необходим поставщик AIS, его ключ и права на хранение и распространение данных.
- **Нет подтверждённой системы геополитической разведки.** Новости не являются проверенными инцидентами, оценками угроз или прогнозами.
- **Нет Forex-котировок в реальном времени.** Курсы валют — ежедневные справочные значения ЕЦБ.
- **Нет облачной или мобильной версии.** Этот репозиторий содержит локальное Python-приложение, а не Node.js/PostgreSQL/PostGIS, Next.js или Expo.

## Дорожная карта

Следующие идеи — **планы, а не реализованные функции**. Реализация зависит от доступности, качества и лицензий источников данных.

1. **Улучшить рыночный терминал:** сохранять историю тиков и строить графики за день/неделю/месяц; добавить CSV-экспорт, уведомления о ценовых уровнях в EUR/CHF и единый статус свежести источников.
2. **Добавить проверенные энергетические данные:** географические трассы трубопроводов и терминалы, цены TTF от разрешённого поставщика с контрактом, сроком, единицей и временем публикации; хранилища, мощности терминалов и погодный контекст.
3. **Показывать коммерческие маршруты СПГ-судов:** только с подходящим AIS-поставщиком и разрешёнными правами использования; реальные позиции не будут выдумываться или экстраполироваться.
4. **Развить геополитическую карту:** объединять GDELT, ACLED и новостные источники, удалять дубликаты, сохранять первоисточник и географию, показывать покрытие и неопределённость. Медийная активность не равна фактическому уровню риска.
5. **Изучить многопользовательский и мобильный доступ:** API, база данных, веб-хостинг или мобильное приложение — только после определения требований к безопасности, аутентификации и стоимости эксплуатации.

Некоторые источники требуют платных подписок и явных прав на хранение и повторное распространение. Симулированные данные не будут выдаваться за реальные.

## Установка

Требуется Python 3.12 (рекомендуется), Git и подключение к Интернету для публичных потоков и источников данных.

```powershell
git clone https://github.com/suvorovpravda/Pravda-Bot.git
Set-Location Pravda-Bot
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
Copy-Item .env.example .env
```

`.env` игнорируется Git. Замените демонстрационные значения; тема ntfy в примере — только заглушка.

## Быстрый запуск

**Windows, запуск одним кликом:** дважды щёлкните `Lancer_Pravda_Bot.bat`. Он запускает бота и веб-панель в отдельных окнах и открывает `http://127.0.0.1:8080`. Не закрывайте окна, пока пользуетесь панелью.

**Вручную**, в двух окнах PowerShell из каталога проекта:

```powershell
# Окно 1
.\.venv\Scripts\Activate.ps1
python main.py
```

```powershell
# Окно 2
.\.venv\Scripts\Activate.ps1
python dashboard.py
```

Откройте [http://127.0.0.1:8080](http://127.0.0.1:8080). Без запущенного бота криптокотировки не поступают. Запустить только панель можно командой `python dashboard.py`. Параметры сервера можно задать, например, так:

```powershell
python dashboard.py --csv .\spread_observations.csv --lng-csv .\lng_observations.csv --port 8081 --threshold 0.05 --stale-after 30
```

Сервер остаётся привязанным к localhost.

## Панель и источники данных

Бот атомарно записывает последние котировки в `market_snapshot.json`; панель опрашивает файл каждую секунду. Через 10 секунд цена считается устаревшей и показывается с соответствующей пометкой.

Приблизительное преобразование криптоцен:

```text
EUR = цена USDT / (USD за 1 EUR)
CHF = EUR × (CHF за 1 EUR)
```

Это оценка по последней сделке и ежедневным курсам ЕЦБ; она не является ценой исполнения или непрерывной Forex-котировкой. График сохраняет тики только во время открытой сессии браузера.

- **ENTSOG:** исходные физические наблюдения с сохранением единиц и периода; точки не дедуплицируются и не равны чистому национальному импорту/экспорту.
- **GDELT / BBC World:** новости со ссылками на издателей; сообщения требуют независимой проверки.
- **TTF:** курс отсутствует, пока не подключён разрешённый источник.
- **Метрики `/api/metrics`:** показатели процесса панели, а не проверка здоровья отдельного процесса бота.

Кэш сервера: потоки ENTSOG — 5 минут, новости — 15 минут, справочник точек — 24 часа, валютные курсы — 6 часов. Источник может обновлять данные медленнее.

Локальные маршруты: `/`, `/api/market`, `/api/fx`, `/api/eu/flows`, `/api/geopolitics`, `/api/gas-price`, `/api/metrics`, `/data`, `/lng-data`. Это не публичный аутентифицированный API.

## Конфигурация

Переменные задаются в `.env` или окружении; уже заданные переменные окружения имеют приоритет.

### Бот и сигналы

| Переменная | По умолчанию | Назначение |
| --- | --- | --- |
| `LM_STUDIO_API_BASE` | `http://192.168.1.146:1234/v1` | Локальный endpoint LM Studio; используйте `http://127.0.0.1:1234/v1`, если сервер на этом компьютере. |
| `LM_MODEL_NAME` | `qwen/qwen3.5-9b` | Идентификатор модели. |
| `TRADING_MODE` | `paper` | Метка отображения; не включает реальные сделки. |
| `MIN_SPREAD_THRESHOLD` | `0.05` | Минимальный валовый спред, проценты. |
| `MAX_ALLOWED_SPREAD_PERCENT` | `5.0` | Предел спреда до классификации как аномалии. |
| `SPREAD_LOG_INTERVAL_SECONDS` | `2` | Интервал записи наблюдений. |
| `DASHBOARD_INTERVAL_SECONDS` | `30` | Интервал обновления терминального Rich-дашборда. |
| `MARKET_DATA_STALE_SECONDS` | `4` | Возраст потока, допустимый при оценке сигнала. |
| `MARKET_DATA_MAX_SKEW_SECONDS` | `3` | Максимальная рассинхронизация потоков. |
| `ALERT_CONFIRMATION_SECONDS` | `3` | Время подтверждения сигнала. |
| `ALERT_REARM_HYSTERESIS_PERCENT` | `0.05` | Падение ниже порога для повторного включения сигнала. |
| `PRICE_ALERT_COOLDOWN_SECONDS` | `5` | Пауза уведомлений по паре; `0` отключает только эту паузу. |
| `NTFY_GLOBAL_COOLDOWN_SECONDS` | `10` | Общая минимальная пауза между уведомлениями. |
| `NTFY_TOPIC_URL` | пусто | HTTPS-адрес темы ntfy; пустое значение отключает уведомления. |

### Бумажный симулятор

`PAPER_MODE=portfolio` использует процент свободного остатка, `PAPER_MODE=fixed` — фиксированный размер.

| Переменная | По умолчанию |
| --- | --- |
| `PAPER_STARTING_BALANCE_USDT` | `1000` |
| `PAPER_POSITION_PERCENT` | `10` |
| `PAPER_FIXED_POSITION_USDT` | `100` |
| `PAPER_EXIT_SPREAD_PERCENT` | `0.05` |
| `PAPER_MAX_HOLD_SECONDS` | `3600` |
| `PAPER_TAKER_FEE_PERCENT` | `0.1` |
| `PAPER_MAX_SLIPPAGE_PERCENT` | `0.5` |
| `PAPER_ORDERBOOK_MAX_AGE_SECONDS` | `2` |

Первые два параметра стакана настраивают комиссию taker **за каждую сторону** и максимальное суммарное проскальзывание; задайте комиссию согласно тарифу своего аккаунта. Максимальный возраст относится ко времени локального получения REST-снимка, а не к гарантированному времени биржи. `estimate_only` означает лишь, что видимая глубина, публичные минимумы и настройки прошли проверку; баланс не проверяется. Частичная оценка не открывает позицию. Комиссии аккаунта, сеть/вывод, задержки, изменение стакана при отправке и движения портфеля не моделируются. Учёт ведётся в USDT, без конвертации в EUR/CHF.
Проверка стакана применяется только при входном сигнале; позиции и их закрытие в `paper_trades.csv` пока рассчитываются по тикерному спреду, а не по фактическим исполнениям. Комиссии аккаунта, сеть/вывод, задержки, изменение стакана при отправке и движения портфеля не моделируются. Учёт ведётся в USDT, без конвертации в EUR/CHF.

### Панель

`DASHBOARD_PORT` / `--port` — `8080`; `SPREAD_LOG_FILE` / `--csv` — `spread_observations.csv`; `LNG_LOG_FILE` / `--lng-csv` — `lng_observations.csv`; `DASHBOARD_STALE_AFTER_SECONDS` / `--stale-after` — `30`; `MIN_SPREAD_THRESHOLD` / `--threshold` — `0.05`; `MARKET_SNAPSHOT_FILE` — `market_snapshot.json`.

Три порога свежести различаются: `MARKET_DATA_STALE_SECONDS` защищает оценку бота, панель рынка считает snapshot устаревшим через 10 секунд, а `DASHBOARD_STALE_AFTER_SECONDS` относится к CSV-журналу.

## Файлы и анализ

Локальные `arbitrage_log.csv`, `spread_observations.csv`, `paper_trades.csv`, `paper_execution_estimates.csv`, `market_snapshot.json`, `lng_observations.csv` и `.env` исключены из Git. Не добавляйте их принудительно, если они содержат личные данные или секреты.

Команды анализа не подключаются к сети:

```powershell
python analyze_spreads.py
python analyze_spreads.py .\spread_observations.csv --output .\spread_summary.csv
python analyze_paper_results.py
```

Скрипты рассчитывают статистику и статусы по сохранённым CSV; это диагностика, а не доказательство исполнимой прибыльности. Детектор использует `abs(Kraken - Binance) / min(Kraken, Binance)`, симулятор — `(Kraken - Binance) / Binance`; эти проценты нельзя напрямую сравнивать. Комиссия 0,36% фиксирована и приблизительна, без учёта проскальзывания и сетевых расходов.

`python lng_observations.py` создаёт демонстрационные строки со статусом `SIMULATED` для TTF, JKM и Henry Hub. Это не текущие котировки и не данные о судах.

## Тесты

```powershell
python -m pytest -q
python -m compileall -q .
git diff --check
```

GitHub Actions запускает проверку импортов/синтаксиса и тесты при push и pull request в `main`.

## Устранение неполадок

- **`ERR_CONNECTION_REFUSED`:** запустите `python dashboard.py`, оставьте окно открытым и обновите `http://127.0.0.1:8080`. Если порт занят, используйте `--port 8081`.
- **Нет или устарели криптоцены:** проверьте `main.py`, Интернет и WebSocket; статус Binance и Kraken показывается отдельно.
- **Нет курсов EUR/CHF:** источник ЕЦБ ежедневный; при недоступности панель не подставляет выдуманное значение.
- **Нет ENTSOG/новостей:** внешние сервисы могут задерживаться или быть недоступны. Для новостей предусмотрен BBC fallback.
- **LM Studio не отвечает:** проверьте URL API и имя модели; для сервера на этом ПК используйте `127.0.0.1`. LLM-анализ необязателен.
- **Не найдены Python/пакеты:** установите Python 3.12 и выполните `python -m pip install -r requirements.txt`.

## Безопасность, структура и участие

Не публикуйте `.env`, ключи или URL ntfy. Темы ntfy не защищены аутентификацией; скрытая URL может быть использована для чтения/публикации. Не открывайте локальный сервер в Интернет без отдельной архитектуры авторизации и защиты. Запросы анализа отправляют выбранной модели символы и цены.

Основные файлы: `main.py` (бот), `dashboard.py` и `dashboard.html` (локальная панель), `live_market_data.py` (snapshot и FX), `energy_data.py` (ENTSOG/GDELT/BBC/TTF), `bot_config.py`, `market_data_health.py`, `alert_gate.py`, `reconnect.py`, `paper_simulator.py`, `security_and_fees.py`, `analyze_spreads.py`, `analyze_paper_results.py`, `lng_observations.py`, `tests/`, `.env.example`, `requirements.txt` и `Lancer_Pravda_Bot.bat`.

Для вклада создайте ветку, добавьте тесты, запустите проверки, проверьте diff на секреты и откройте pull request с описанием ограничений. Для TTF и AIS укажите поставщика, права и точность данных.

**Лицензия:** файл лицензии в репозитории отсутствует. Не предполагайте, что разрешено свободное повторное использование или распространение.

<a id="english"></a>

# English

**Pravda-Bot** is a local Python project for monitoring crypto markets and European energy data. It consumes public Kraken and Binance ticker streams, measures spreads, records observations for analysis, and serves a local web dashboard.

> **Important:** the bot does not place real orders and does not need exchange API keys. Signals use last-trade prices. On a confirmed signal, a separate estimate fetches public order books and models visible depth, VWAP, configured taker fees, slippage, and public market minimums. It does not verify balances, spot inventory, margin, borrowing, actual account fees, execution latency, or transfers, so it cannot prove a trade is executable. This project is not financial advice.

## Features

### Crypto monitoring

- Public **Kraken** and **Binance** WebSocket tickers for BTC/USDT, ETH/USDT, and SOL/USDT.
- Detection of disconnected, stale, or desynchronized streams, with exponential-backoff reconnection.
- Gross-spread calculations, approximate fees, configurable thresholds, and alert confirmation/hysteresis.
- Price-movement and opportunity notifications through ntfy when configured.
- Optional analysis of eligible signals via a local LM Studio-compatible server.
- Portfolio or fixed-size paper simulator; no live trades are submitted.
- On confirmed signals, estimates market orders against public Kraken/Binance books, including depth, VWAP, partial fills, taker fees, slippage, and CCXT amount precision/limits. Rejected, partial, or unavailable estimates do not open a new paper position.

### Local dashboard

- Bot-fed crypto quotes and a chart of ticks seen since the page was opened.
- Crypto prices converted and displayed in **EUR and CHF**, using daily ECB reference rates distributed by Frankfurter and the indicative assumption USDT ≈ USD.
- Per-feed status and age; stale prices are identified and must not be treated as current.
- ENTSOG cross-border physical-flow observations and GDELT news, with BBC World RSS fallback.
- French, Russian, and English UI, with the selected language saved in the browser; blue, white, and red palette.
- The server binds to `127.0.0.1` by default.

## What this project does not do

- **No live order placement:** monitoring and paper simulation are not an automated trading system.
- **No account validation:** public books do not reveal balances or spot inventory. The estimate marks these as unverified and does not model margin, borrowing, transfers, account-specific risk limits, or order-submission latency.
- **No TTF quote:** no licensed market-price provider is connected. The dashboard therefore shows neither a TTF price nor a quote unit as if one were available. The ICE link describes contract specifications; it is not a price feed.
- **No actual pipeline-route map:** the ENTSOG schematic shows cross-border observation points, not pipeline paths.
- **No LNG carrier AIS tracking:** vessel positions and trade routes require a suitable AIS provider, credentials, and storage/redistribution rights.
- **No verified geopolitical intelligence:** news reports are not verified incidents, threat assessments, or predictions.
- **No tick-by-tick Forex:** EUR/CHF and other fiat rates are daily ECB reference rates.
- **No cloud or mobile app:** this repository is a local Python application, not a Node.js/PostgreSQL/PostGIS, Next.js, or Expo project.

## Roadmap

The following items are **ideas, not implemented features**. Delivery depends on data availability, quality, licensing, and user needs.

1. **A more useful market terminal:** persist tick history; add day/week/month charts, CSV export, EUR/CHF threshold alerts, and unified source-freshness status.
2. **Verified energy data:** geographically accurate pipeline and terminal layers; licensed TTF quotes with contract, expiry, unit, and timestamp; storage, LNG capacity, and relevant weather context.
3. **LNG carrier routes:** display routes only after integrating a suitable AIS provider under its terms and redistribution rights; never invent or extrapolate live positions.
4. **A careful geopolitical map:** combine GDELT, ACLED, and news providers; deduplicate while retaining source links, locations, coverage, and uncertainty. Media volume must not be represented as ground-truth risk.
5. **Multi-device access:** consider an API/database, hosted web app, or mobile client only after defining authentication, data protection, and operating costs.

Some sources require paid subscriptions and explicit rights to store or redistribute their data. Simulated data will not be presented as live observations.

## Requirements and installation

Python 3.12 is recommended (the codebase requires Python 3.10+), with Git and Internet access for public market and data sources.

```powershell
git clone https://github.com/suvorovpravda/Pravda-Bot.git
Set-Location Pravda-Bot
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
Copy-Item .env.example .env
```

`.env` is ignored by Git. Replace example values; the ntfy URL in the example is only a placeholder.

## Quick start

**Windows one-click launch:** double-click `Lancer_Pravda_Bot.bat`. It starts the bot and dashboard in separate windows and opens `http://127.0.0.1:8080`. Keep both windows open while using the dashboard.

**Manual launch**, in two PowerShell windows from the repository directory:

```powershell
# Window 1
.\.venv\Scripts\Activate.ps1
python main.py
```

```powershell
# Window 2
.\.venv\Scripts\Activate.ps1
python dashboard.py
```

Open [http://127.0.0.1:8080](http://127.0.0.1:8080). Crypto quotes require the bot to be running. To run only the dashboard, use `python dashboard.py`. For example, customize its paths and settings with:

```powershell
python dashboard.py --csv .\spread_observations.csv --lng-csv .\lng_observations.csv --port 8081 --threshold 0.05 --stale-after 30
```

The web server remains bound to localhost.

## Dashboard and data sources

The bot atomically writes recent quotes to `market_snapshot.json`; the dashboard polls that file every second. Quotes older than 10 seconds are marked stale.

Indicative crypto conversion:

```text
EUR = USDT price / (USD per 1 EUR)
CHF = EUR × (CHF per 1 EUR)
```

These are last-trade prices converted with daily ECB reference rates, not executable prices or streaming FX quotes. The chart retains ticks only for the current browser session.

- **ENTSOG:** source physical-flow observations retain their original units and periods; they are not deduplicated or equivalent to national net imports/exports.
- **GDELT / BBC World:** linked news reports require independent verification.
- **TTF:** no quote is shown until an authorized, verifiable provider is configured.
- **`/api/metrics`:** metrics for the dashboard process, not a health check for the separate bot.

Server cache periods: ENTSOG flows, 5 minutes; news, 15 minutes; ENTSOG point reference data, 24 hours; FX rates, 6 hours. Upstream publication may be slower.

Local routes: `/`, `/api/market`, `/api/fx`, `/api/eu/flows`, `/api/geopolitics`, `/api/gas-price`, `/api/metrics`, `/data`, and `/lng-data`. This is not a public or authenticated API.

## Configuration

Settings may be placed in `.env` or in the process environment; existing environment variables take precedence.

### Bot and alerts

| Variable | Default | Purpose |
| --- | --- | --- |
| `LM_STUDIO_API_BASE` | `http://192.168.1.146:1234/v1` | Local LM Studio-compatible API; use `http://127.0.0.1:1234/v1` when it runs on this PC. |
| `LM_MODEL_NAME` | `qwen/qwen3.5-9b` | Model identifier. |
| `TRADING_MODE` | `paper` | Display label only; does not enable live trading. |
| `MIN_SPREAD_THRESHOLD` | `0.05` | Minimum gross spread, percent. |
| `MAX_ALLOWED_SPREAD_PERCENT` | `5.0` | Maximum spread before anomaly rejection. |
| `SPREAD_LOG_INTERVAL_SECONDS` | `2` | Per-symbol observation interval. |
| `DASHBOARD_INTERVAL_SECONDS` | `30` | Rich terminal dashboard refresh interval. |
| `MARKET_DATA_STALE_SECONDS` | `4` | Maximum feed age for evaluating a signal. |
| `MARKET_DATA_MAX_SKEW_SECONDS` | `3` | Maximum desynchronization between feeds. |
| `ALERT_CONFIRMATION_SECONDS` | `3` | Required signal confirmation duration. |
| `ALERT_REARM_HYSTERESIS_PERCENT` | `0.05` | Drop below the trigger before rearming. |
| `PRICE_ALERT_COOLDOWN_SECONDS` | `5` | Per-symbol alert cooldown; `0` disables only this cooldown. |
| `NTFY_GLOBAL_COOLDOWN_SECONDS` | `10` | Minimum global interval between ntfy sends. |
| `NTFY_TOPIC_URL` | empty | HTTPS ntfy topic URL; empty disables notifications. |

### Paper simulator

`PAPER_MODE=portfolio` sizes positions as a percentage of available cash; `PAPER_MODE=fixed` uses a fixed size.

| Variable | Default |
| --- | --- |
| `PAPER_STARTING_BALANCE_USDT` | `1000` |
| `PAPER_POSITION_PERCENT` | `10` |
| `PAPER_FIXED_POSITION_USDT` | `100` |
| `PAPER_EXIT_SPREAD_PERCENT` | `0.05` |
| `PAPER_MAX_HOLD_SECONDS` | `3600` |
| `PAPER_TAKER_FEE_PERCENT` | `0.1` |
| `PAPER_MAX_SLIPPAGE_PERCENT` | `0.5` |
| `PAPER_ORDERBOOK_MAX_AGE_SECONDS` | `2` |

The fee and slippage settings apply per taker leg and cumulatively across both legs, respectively; set fees to your account's tier. Book age is measured from local REST response receipt, not a guaranteed exchange timestamp. `estimate_only` means only that visible depth, public minimums, and configured limits passed; balances remain unverified. Partial estimates do not open a position. The book check applies only at entry; positions and closures in `paper_trades.csv` still use ticker spread rather than actual book fills. Account-specific fees, network/withdrawal fees, latency, book changes during submission, and portfolio movements are not modeled. The simulator accounts in USDT, not dashboard EUR/CHF.

### Dashboard

`DASHBOARD_PORT` / `--port`: `8080`; `SPREAD_LOG_FILE` / `--csv`: `spread_observations.csv`; `LNG_LOG_FILE` / `--lng-csv`: `lng_observations.csv`; `DASHBOARD_STALE_AFTER_SECONDS` / `--stale-after`: `30`; `MIN_SPREAD_THRESHOLD` / `--threshold`: `0.05`; `MARKET_SNAPSHOT_FILE`: `market_snapshot.json`.

Freshness settings differ: `MARKET_DATA_STALE_SECONDS` protects bot signal evaluation; the market snapshot is stale after 10 seconds; `DASHBOARD_STALE_AFTER_SECONDS` applies to the spread CSV.

## Files and offline analysis

Local `arbitrage_log.csv`, `spread_observations.csv`, `paper_trades.csv`, `paper_execution_estimates.csv`, `market_snapshot.json`, `lng_observations.csv`, and `.env` are ignored by Git. Do not force-add local files containing private data or secrets.

Analysis scripts do not connect to the network:

```powershell
python analyze_spreads.py
python analyze_spreads.py .\spread_observations.csv --output .\spread_summary.csv
python analyze_paper_results.py
```

They report historical statistics and CSV statuses for diagnostics; they do not establish executable profitability. The detector uses `abs(Kraken - Binance) / min(Kraken, Binance)`, whereas the simulator uses `(Kraken - Binance) / Binance`; their percentages are not directly comparable. The fixed 0.36% fee estimate excludes slippage and network costs.

`python lng_observations.py` writes **SIMULATED** sample rows for TTF, JKM, and Henry Hub. It does not fetch current prices or vessel data.

## Tests

```powershell
python -m pytest -q
python -m compileall -q .
git diff --check
```

GitHub Actions also runs syntax/import checks and tests on pushes and pull requests to `main`.

## Troubleshooting

- **`ERR_CONNECTION_REFUSED`:** start `python dashboard.py`, keep its window open, and reload `http://127.0.0.1:8080`. If the port is occupied, use `--port 8081`.
- **Missing or stale crypto quotes:** check that `main.py` is running and that Internet/WebSocket access is available. Binance and Kraken statuses are shown separately.
- **Missing EUR/CHF rates:** ECB rates are daily; if the source is unavailable, the dashboard does not invent a value.
- **ENTSOG/news unavailable:** upstream services may be delayed or offline; the BBC feed is a news fallback.
- **LM Studio unavailable:** check the API URL and model identifier; use `127.0.0.1` if it is on this PC. LLM analysis is optional.
- **Python/dependencies missing:** install Python 3.12 and run `python -m pip install -r requirements.txt`.

## Security, repository structure, and contributing

Never publish `.env`, credentials, or an ntfy topic URL. ntfy topics do not provide authentication; anyone who knows the URL may read or publish messages. Do not expose the local web server to the Internet without separate authentication and security controls. LLM analysis sends the selected symbols and prices to the configured model.

Key files: `main.py` (bot), `dashboard.py` and `dashboard.html` (local dashboard), `live_market_data.py` (snapshot and FX), `energy_data.py` (ENTSOG/GDELT/BBC/TTF), `bot_config.py`, `market_data_health.py`, `alert_gate.py`, `reconnect.py`, `paper_simulator.py`, `security_and_fees.py`, `analyze_spreads.py`, `analyze_paper_results.py`, `lng_observations.py`, `tests/`, `.env.example`, `requirements.txt`, and `Lancer_Pravda_Bot.bat`.

To contribute, create a branch, add tests, run checks, review the diff for secrets, and open a pull request describing the change and limitations. TTF/AIS integrations must document providers, licensing, and data accuracy.

**License:** no license file is currently present. Do not assume the code is freely reusable or redistributable.
