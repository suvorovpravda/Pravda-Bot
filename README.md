# Pravda-Bot

## Français

**Observer les marchés, conserver les preuves et tester des hypothèses sans passer d’ordres.**

Pravda-Bot est un observatoire Python local : tickers publics Kraken/Binance pour BTC/USDT, ETH/USDT et SOL/USDT, comparaison de prix, simulateur papier, tableau de bord et historique SQLite. Il permet d'étudier la qualité des flux, les spreads après frais simplifiés et les erreurs de traitement asynchrone. Les connexions privées et le trading réel ne sont pas implémentés.

### État de cette branche

Cette correction part du `main` public `e292a37` et réintègre le véritable historique SQLite du ZIP transmis par Vincent : historique persistant, alertes locales, exports journaliers et lanceur Windows. Les artefacts privés du ZIP (`.env`, CSV, SQLite, venv et EXE) sont exclus. Les nouveaux modules `geospatial_mapper.py`, `maritime_proxy.py`, `osint_pipeline.py`, `ais_stream_connector.py` et `enhanced_terminal.py` du dépôt sont conservés. Leur présence ne démontre pas une connexion réelle, une licence de données ou une validation opérationnelle. Le dashboard récent est conservé ; les endpoints AIS annoncent une indisponibilité tant que leur fournisseur est un stub.

**Branche candidate, vérification en cours.** Les résultats locaux seront rapportés pour le commit concerné ; la CI Windows/Linux et la revue Gemini restent distinctes. Une PR en brouillon n'est pas une release validée. La moitié de l'équipe est en vacances ; reprise progressive prévue vers le **2 novembre 2026**.

### Fonctions et limites

| Fonction | Utilité | Limite |
|---|---|---|
| Tickers et spread | Observer les écarts entre deux places | Un dernier prix n'est pas un prix d'exécution garanti |
| Simulation papier | Comparer des hypothèses et journaliser des positions virtuelles | État du portefeuille en mémoire ; remise à zéro au redémarrage |
| Historique SQLite | Courbes et exports des observations conservées | Ne transforme pas une observation passée en signal actuel ; stockage à surveiller |
| Carnets publics | Estimation de liquidité et de frais | Aucun solde, inventaire, emprunt, transfert ou placement d'ordre validé |
| ENTSOG | Observations de flux aux points transfrontaliers | Pas de tracés réels de conduites ni preuve d'état opérationnel |
| BCE / Frankfurter | Conversions EUR/CHF à taux de référence | Données quotidiennes ; pas de Forex tick par tick ; USDT n'est pas garanti égal à USD |
| GDELT / BBC | Consulter des titres rapportés | Pas d'incidents vérifiés ni de prédictions géopolitiques |
| LM Studio | Analyse textuelle locale facultative | Modèle à charger séparément ; aucun fallback cloud automatique |

Aucun fournisseur de cours TTF licencié ni suivi AIS opérationnel vérifié n'est intégré dans cette copie. Le lien ICE décrit un contrat, pas un prix. Les projets GEM/AIS, Kafka Docker Playground et Spotify restent des pistes à rapprocher d'une sauvegarde vérifiable avant réintégration. Pas de promesse de confidentialité totale, de continuité garantie ou de performance comparable aux modèles cloud.

Les frais simplifiés restent **0,36 %** : `0.0036` en fraction, `0.36` en points de pourcentage. Exemple : `1.20 % - 0.36 point = 0.84 %` net estimé. Le détecteur d'écart absolu et le spread directionnel d'achat/vente sont distincts ; un spread directionnel négatif conserve son signe. Le plafond brut par défaut de 5 % rejette des anomalies ; il ne prouve pas qu'un arbitrage est réalisable. Les coûts fixes de réseau/retrait et un véritable modèle de remplissage restent absents.

### Installation et lancement

Cible : Python 3.12, Windows principal et Linux de validation. Utiliser `.venv`, attendu par le lanceur. Dans la copie choisie du dépôt :

```powershell
py -3.12 -m venv .venv
.\.venv\Scripts\python.exe -m pip install --require-hashes -r requirements-bootstrap.txt
.\.venv\Scripts\python.exe -m pip install --require-hashes -r requirements.txt
.\.venv\Scripts\python.exe -m pip check
if (-not (Test-Path '.env')) { Copy-Item '.env.example' '.env' }
$env:NTFY_TOPIC_URL=''
.\.venv\Scripts\python.exe -m unittest discover -s tests -t . -v
```

Démarrer `main.py` et `dashboard.py --port 8080` avec ce même Python, dans deux terminaux. Ouvrir http://127.0.0.1:8080/. Laisser LM Studio désactivé pour un essai sans modèle. Sous Linux : `python3.12 -m venv .venv`, puis `.venv/bin/python` pour les mêmes commandes.

`Pravda_Lanceur.exe` reste un lanceur : il utilise les sources et le `.venv` voisins. Vérifier le dossier réellement lancé ; une copie `.copilot/repos` n'est pas automatiquement synchronisée. Le code du lanceur est revu ; aucun nouvel EXE Windows n'est annoncé comme compilé par une simple validation Linux. Les builds et audits utilisent des environnements séparés.

### Sécurité et vérifications

Le dashboard reste sur **127.0.0.1**, sans authentification : ne pas l'exposer sur Internet. Le serveur vérifie Host/Origin, borne les requêtes et les lectures CSV, limite les requêtes et renvoie des erreurs génériques. La limite est configurable : le polling de l'interface de ce ZIP dépasse 60 requêtes/minute, d'où un défaut de 240 par IP. Les exports de texte externe sont protégés contre les formules de tableur.

`secure_http.py` applique des hôtes exacts, HTTPS public, l'exception HTTP loopback pour LM Studio, des délais, des réponses lues sous plafond et des redirections interdites. Les POST ne sont pas rejoués. La vérification DNS préalable n'est pas un verrouillage de l'IP de connexion ; CCXT et WebSockets conservent leurs transports propres. Les secrets, logs et fichiers de données restent locaux. Une vérification de dépendances ne garantit pas l'absence de code malveillant.

```text
python -m unittest discover -s tests -t . -v
python -m unittest discover -s launcher_tests -v
python -m compileall -q -x "(^|[/\\])(.git|.venv|venv|build|dist)([/\\]|$)" .
python -m pip check
```

La CI proposée contrôle la syntaxe de tous les modules publiés, y compris `src`, les tests, les dépendances et leurs audits. Validation locale du candidat : 177 tests du projet réussis sous Linux ; ce résultat ne remplace pas la CI du SHA qui sera publié. Le diagnostic du retard STALE de 25–29 s demande encore une mesure des flux réels ; les régressions simulées ne mesurent pas le transit réseau.

### Organisation agentique et participation

Vincent fixe les objectifs et accepte les travaux. **GPT : maîtrise d'ouvrage technique ; Gemini : pilotage et première validation ; vérificateur : défauts reproductibles ; comparateur : versions et preuves ; implémenteur : corrections et tests.** La boucle est correction → vérificateur → comparateur → premier visa Gemini → validation GPT → livraison et contrôle Vincent. Une approbation non obtenue n'est jamais attribuée à un modèle. Cette organisation de développement n'est pas une architecture d'agents autonomes dans le bot.

Les contributions recherchées : bugs reproductibles, tests asynchrones, accessibilité, traductions, qualité des données et connecteurs autorisés. Commencer par une issue et une PR limitée, avec provenance et limites. Le projet peut servir de base à d'autres observatoires locaux et outils de fiabilité des données. [Kafka Docker Playground](https://github.com/vdesabou/kafka-docker-playground), proposé par le cousin du porteur du projet, peut aider à expérimenter production/consommation/rejeu ; il ne fournit ni données ni correction automatique de latence. Aucune licence générale du code n'est supposée ; vérifier les licences des composants avant redistribution.

## Русский

Pravda-Bot — локальный Python-инструмент наблюдения за публичными котировками Kraken/Binance, бумажной симуляции и анализа качества данных. Архив добавляет SQLite-историю, уведомления внутри интерфейса, CSV-экспорт и Windows-лаунчер. Реальные ордера, приватные счета, лицензированные цены TTF, AIS и непрерывный Forex в этой копии не реализованы. ENTSOG публикует наблюдения в точках, BCE — ежедневные справочные курсы; новости не являются подтверждёнными происшествиями.

Комиссия модели — 0,36 процентного пункта; порог аномалий — 5 %. Это упрощённая оценка, а не гарантия доходности или исполнения. Бумажный портфель обнуляется при перезапуске; SQLite хранит историю наблюдений. Старая локальная модернизация была потеряна при обслуживании временного хранилища и не считается частью этой ветки без восстановления и проверки.

Python 3.12, `.venv`, установка bootstrap/runtime с `--require-hashes`, затем `pip check` и тесты. Dashboard — только 127.0.0.1 без аутентификации. LM Studio необязателен, только loopback, без облачного fallback. HTTP-политики ограничивают адреса, перенаправления, размер и время ответов; DNS-проверка не закрепляет IP соединения. Секреты, БД, журналы и EXE не публикуются. Статус — локальный кандидат на базе `e292a37`: 177 тестов прошли; CI и независимая проверка ещё требуют подтверждения на соответствующем SHA. Новые модули географии, AIS и OSINT сохранены; stub истории заменён настоящим SQLite-модулем.

Половина команды в отпуске, возвращение к работе ожидается около **2 ноября 2026 года**. Vincent определяет цели и принимает результат; GPT координирует техническую работу, Gemini руководит проектом, проверяющий и сравнивающий агенты оценивают доказательства, исполнитель исправляет код. Подтверждённые ошибки, тесты и полезные коннекторы приветствуются; происхождение данных и ограничения обязательны.

## English

Pravda-Bot is a local Python observatory for public Kraken/Binance tickers, paper simulation and data-quality analysis. The supplied archive adds SQLite history, local dashboard alerts, CSV reports and a Windows launcher. This copy has no real orders, private-account validation, licensed TTF prices, AIS feed or streaming Forex. ENTSOG observations are points, ECB rates are daily references, and reported news is not verified intelligence.

The simplified fee estimate subtracts 0.36 percentage points; the gross anomaly cap defaults to 5%. Neither establishes profitability or executable fills. Paper portfolio state resets on restart; SQLite retains observations. Earlier unpushed modernization was lost during temporary-workspace maintenance and is not claimed as part of this branch without recovery and verification.

Use Python 3.12 and `.venv`; install the hashed bootstrap and runtime, then run `pip check` and both test suites. The unauthenticated dashboard must stay on 127.0.0.1. LM Studio is optional, loopback-only, without a cloud fallback. HTTP policies restrict destinations, redirects, time and response sizes; a preliminary DNS check does not pin the connection IP. Secrets, runtime databases/logs and executables are excluded from Git. This is a local candidate on `e292a37`; 177 project tests passed. The newer geography, AIS, OSINT and terminal modules are retained; the broken history stub is replaced with the real SQLite implementation. Remote CI and independent review must be confirmed for the actual published SHA.

Half the team is on holiday; work is expected to resume around **2 November 2026**. Vincent sets direction and accepts the work; GPT coordinates technical ownership, Gemini manages the project, verifiers/comparators examine evidence, and implementers provide fixes and regressions. Contributions should include reproducible examples, tests, provenance and limits. This foundation can support other local observatories and data-reliability projects. Kafka Docker Playground is under consideration, not deployed.
