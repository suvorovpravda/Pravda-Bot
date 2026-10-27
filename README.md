# Pravda-Bot

**Surveillance locale des marchés crypto, énergie et géopolitique — sans cloud, sans ordre réel**

Pravda-Bot est une application Python locale qui agrège des flux publics de marché, d’énergie et d’actualités, les valide, les corrèle et les expose dans un tableau de bord web local. Conçue pour un usage personnel et de recherche, elle fonctionne entièrement sur votre machine : pas de compte d’échange, pas de clé API, pas de données envoyées à l’extérieur.

> **Avertissement** : surveillance publique uniquement, simulation papier uniquement. Aucun ordre réel n’est passé. Les estimations d’exécution ne valident pas les soldes, l’inventaire, la marge ou la latence réelle.

## Pourquoi ce projet est complexe

Pravda-Bot combine plusieurs domaines techniques et sources hétérogènes avec des contraintes fortes :

* **Flux temps réel multi-sources** : WebSockets Kraken/Binance, REST ENTSOG, GDELT/BBC RSS, Frankfurter BCE. Chaque source a sa propre latence, son propre format et ses propres périodes de disponibilité.
* **Cohérence et fraîcheur** : horodatage monotone, détection de désynchronisation, invalidation des actions en attente, reconnexion avec backoff exponentiel.
* **Validation stricte** : refus des booléens comme prix, timestamps futurs, spreads non finis, checksum AIS, validation de schéma Pydantic.
* **Sécurité locale** : bind `127.0.0.1` uniquement, whitelist de domaines, pas de `shell=True`, pas de secrets dans les logs, `.env` jamais versionné.
* **Simulation papier** : estimation d’exécution sur carnet public avec VWAP, frais taker, slippage, limites de quantité. Aucune validation de compte.
* **Tableau de bord sans dépendance externe** : `http.server` natif, interface moderne, état des sources live/delayed/stale/unavailable, conversions EUR/CHF avec âge du taux.

Cette complexité est volontaire : éviter les hallucinations de données, signaler explicitement l’indisponibilité plutôt que simuler, et garder une traçabilité complète source → règle → affichage.

## Fonctionnalités clés

### Marchés crypto
- Flux WebSocket publics Kraken/Binance pour BTC/USDT, ETH/USDT, SOL/USDT
- Détection déconnexion, gaps, désynchronisation inter-exchanges
- Calcul spread brut, frais 0.36 %, seuils configurables, hystérésis et confirmation
- Alertes ntfy locales, analyse LLM optionnelle via LM Studio local
- Simulateur papier portefeuille ou taille fixe
- Estimation d’exécution au signal confirmé sur carnet public

### Énergie & actualités
- Observations ENTSOG transfrontalières, cache LRU borné
- Actualités GDELT avec repli BBC World RSS
- Taux BCE via Frankfurter, conversions EUR/CHF indicatives
- Carte schématique des points ENTSOG, données GEM pour projets gaziers

### Tableau de bord local
- Serveur HTTP sur `127.0.0.1:8080`
- Prix crypto, EUR/CHF, état des sources, actualités, carte énergie
- Interface FR/RU/EN, persistance locale du choix de langue
- Pas d’authentification, usage local uniquement

## Ce que le projet ne fait PAS, et pourquoi
- **Pas d’ordres réels, pas de clés d’échange** : surveillance publique uniquement, simulation papier uniquement.
- **Pas de cours TTF licencié, pas de Forex tick par tick** : les taux utilisés sont les références BCE quotidiennes via Frankfurter, pas un flux interbancaire en temps réel. Aucun fournisseur de prix de marché TTF n’est connecté.
- **Pas de carte de tracés de pipelines réels, pas de suivi AIS sans fournisseur autorisé** : la visualisation ENTSOG montre des points transfrontaliers publiés, pas des tracés de conduites. Le suivi AIS de navires GNL nécessite un fournisseur autorisé avec clés et droits de redistribution.
- **Pas de renseignement géopolitique opérationnel, les articles restent à vérifier** : les actualités GDELT/BBC sont des informations rapportées, pas des incidents vérifiés ni des évaluations de menace.

Ces limites sont volontaires : elles préservent la simplicité locale, évitent les dépendances licenciées et garantissent la traçabilité source → règle → affichage.

## Démarrage rapide Windows
```powershell
git clone https://github.com/suvorovpravda/Pravda-Bot.git
cd Pravda-Bot
py -3.12 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
Copy-Item .env.example .env
```
Double-cliquez sur `Lancer_Pravda_Bot.bat` → bot + dashboard + navigateur.

Manuel :
```powershell
python main.py
python dashboard.py
```
Accédez à http://127.0.0.1:8080

## Installation propre et validation
```powershell
py -3.12 -m venv .venv
.venv\Scripts\python.exe -m pip install --require-hashes -r requirements.txt
.venv\Scripts\python.exe -m pip check
.venv\Scripts\python.exe -m unittest discover -s tests -t . -v
```
106 tests passent sur une installation propre Python 3.12.

## Architecture de sécurité & fondations roadmap
La sécurité n’est pas une section isolée, c’est la fondation qui rend les évolutions futures possibles sans réécrire l’architecture.

### Pourquoi c’est efficace aujourd’hui
- **Exécution locale stricte** : bind `127.0.0.1` uniquement, pas d’exposition réseau. Le dashboard ne peut pas être atteint depuis le réseau, ce qui élimine la surface d’attaque externe par défaut.
- **Pas de secrets en code** : `.env` ignoré par Git, `.env.example` versionné, `NTFY_TOPIC_URL` forcé vide au démarrage. Aucun secret ne fuite dans les logs.
- **Contrôle des entrées et des sorties** : whitelist de domaines, `TLS verify=True`, pas de `shell=True`, pas de `eval/exec`. Blocage SSRF et injection de commandes.
- **Intégrité des dépendances** : `requirements.in` → `pip-compile --generate-hashes` → `requirements.txt` hashé, `pip-audit` en CI, `compileall` et tests unitaires. Toute dérive de paquet est détectée.
- **Traçabilité et validation** : schémas Pydantic, horodatage monotone, invalidation des données périmées. On signale l’indisponibilité plutôt que de simuler.

### À quoi ça sert maintenant
- Surveillance fiable de sources publiques sans risque d’exposition de clés.
- Simulation papier sans risque d’ordre réel.
- Tableau de bord local utilisable en environnement de travail sensible.
- CI reproductible et auditable pour une équipe distribuée.

### Comment cela permet des projets plus complexes
- **Base pour multi-appareils** : la séparation stricte réseau / secrets / validation permet d’ajouter plus tard une API authentifiée, une base de données chiffrée et un serveur hébergé sans réécrire les garde-fous.
- **Intégrations licenciées** : AIS, prix TTF, flux propriétaires nécessitent gestion de clés, rotation et audit. L’architecture actuelle avec variables d’environnement validées, audit de dépendances et logs sans secret est directement réutilisable.
- **Conformité et gouvernance** : traçabilité source → règle → affichage, journal de provenance et tests automatisés facilitent la certification et l’audit externe.
- **Évolutivité** : le modèle de fraîcheur des sources, de backoff exponentiel et de validation stricte est extensible à des centaines de flux sans dégradation de la fiabilité.

Cette base sécurisée est la condition préalable aux priorités de la feuille de route : historique persistant chiffré, alertes EUR/CHF, carte d’infrastructures, et veille géopolitique avec provenance.

## Feuille de route
Voir [ROADMAP.md](ROADMAP.md)

Priorité 1 : historique prix persistant, prix EUR/CHF lisibles, état sources unifié, résumé quotidien exportable  
Priorité 2 : carte infrastructures gazières vérifiées, AIS, prix TTF vérifié, contexte énergétique  
Priorité 3 : veille géopolitique prudente avec provenance et indicateurs explicites

## Structure
- `main.py` : orchestration flux, dispatcher, simulation papier
- `dashboard.py` : serveur HTTP local
- `live_market_data.py` : adaptateurs marché
- `energy_data.py` : ENTSOG, GDELT/BBC
- `bot_config.py` : validation config
- `tests/` : 106 tests unitaires

## Licence
MIT. Données tierces sous leurs licences respectives. Voir `THIRD_PARTY_NOTICES.md`.

---

*Application locale, surveillance publique, simulation papier uniquement. La moitié de l’équipe est en vacances, reprise progressive vers le 2 novembre 2026.*
