# Pravda-Bot — campagne quantitative Rust hors réseau

## Périmètre et règles de sécurité

`pravda-harness.exe` est un binaire Windows Rust x64 autonome **après compilation**. Il orchestre des exécutables Rust de simulation déterministe. Il ne lance ni Python, ni Node, ni PowerShell pour exécuter les campagnes. La CI GitHub utilise encore des actions et scripts d'infrastructure pour compiler et déposer les artefacts ; ceci ne constitue **pas** une garantie que tout le dépôt historique est 100 % Rust.

**Isolation impérative :** exécuter dans une VM ou un poste isolé dont **tous les accès réseau sortants et entrants sont bloqués par le système d'exploitation ou l'hyperviseur**, indépendamment des variables d'environnement. Les enfants sont des programmes natifs ; définir `PRAVDA_OFFLINE_TEST=true` ne les empêche pas physiquement d'ouvrir des sockets. Ne pas monter de dossiers contenant des clés réelles ou de données bancaires. Stocker le binaire de replay et son manifeste dans un répertoire à accès restreint ; vérifier les empreintes SHA-256.

## Démarrage immédiat avec la simulation intégrée

Récupérer l'artefact `pravda-harness-win64-unsigned-test` depuis le workflow GitHub Actions validé de la PR #99, décompresser puis ouvrir un terminal Windows dans le dossier. Les exemples suivants utilisent **exclusivement** le programme compilé :

```text
pravda-harness.exe --smoke
pravda-harness.exe prepare . pravda-harness.exe 1000 campaign.json --fixture-ok
pravda-harness.exe run campaign.json .
```

- `--smoke` vérifie succès, rejet de drawdown et timeout : son `build_accepted` doit valoir `false` **par conception**. Le code de sortie doit rester 0 si les trois cas produisent les résultats attendus.
- `prepare` génère un manifeste versionné, strict et SHA-256 vérifié sans modifier le programme testé ; le fichier `campaign.json` doit ne pas déjà exister.
- `run` exécute 1 000 fois le **fixture synthétique** `--fixture-ok`. Il ne teste **pas** la stratégie réelle, la connectivité de marché ou les performances économiques.
- Pour une campagne réelle, remplacer `pravda-harness.exe` par le **binaire Rust de replay hors réseau** qui émet une seule structure `Metrics` JSON conforme sur `stdout` et se termine avec un code de sortie 0. Les données doivent venir de fixtures de marché locales figées, jamais du réseau.

Le résultat est une unique ligne de JSON compact : `schema_version`, `build_accepted`, `aggregate`, `rejected_reasons`. Copier cette ligne dans ChatGPT pour itérer sur la stratégie et les tests.

## Contrat du replay

Chaque enfant reçoit `PRAVDA_RUN_ID`, `PRAVDA_OFFLINE_TEST=true`, `PRAVDA_MARKET_FEEDS_ENABLED=false`, `PRAVDA_LIVE_TRADING_ENABLED=false` et `PRAVDA_REVOLUT_X_ENABLED=false`. L'environnement d'origine est effacé, hormis `SystemRoot` sous Windows. Aucun secret d'exchange n'est transmis. Les métriques sont structurées dans `rust/pravda-server/src/harness_metrics.rs` et utilisent `#[serde(deny_unknown_fields)]`.

Unités : PnL en centimes USD, commissions en centimes, drawdown et VaR en points de base, p50/p99 en microsecondes, slippage en points de base. Le validateur impose le rapprochement `gross - fees = net`, les valeurs finies et des ratios cohérents, puis les seuils de risque. Le schéma ne transporte pas les courbes de PnL détaillées, les trades individuels, les données personnelles ni les logs.

Les seuils par défaut sont **provisoires** : 200 transactions minimum par run, PnL net >= 0, maxDD <= 1500 bps, VaR 99 <= 400 bps, slippage <= 15 bps, p99 <= 25 000 microsecondes et ratios risque/rendement non négatifs. Le desk doit les approuver, documenter la méthode VaR/Sharpe/Sortino/Calmar, la fréquence d'échantillonnage et les coûts de marché avant toute publication de production. Des performances synthétiques ne prouvent aucune rentabilité future.

## Résilience et limites à traiter avant un label « institutionnel »

- Le timeout tue et récolte le processus enfant ; les sous-processus éventuellement engendrés ne sont pas tous nécessairement tués. La production nécessite des **Windows Job Objects** / cgroups pour la terminaison de toute l'arborescence.
- La mesure RSS échantillonnée ne garantit pas l'observation de pics très brefs ou d'une fuite mémoire de descendants. Le durcissement exige des limites mémoire kernel.
- `stdout` / `stderr` sont drainés et conservés en mémoire dans une limite fixe, sans être publiés. Seuls des codes d'erreur autorisés sont rapportés.
- Le manifeste interdit les chemins arbitraires et contrôle les SHA-256, mais **un binaire autorisé doit lui-même être audité**. Des métriques inventées par un replay compromis ne peuvent pas être détectées uniquement par un schéma JSON.
- `mean_sharpe` est la moyenne arithmétique des Sharpe *par run* ; ce n'est pas le Sharpe d'un portefeuille consolidé. Pour une mesure consolidée, recalculer depuis une série de rendements horodatés auditée.
- Les checks GitHub, la revue obligatoire et l'isolement machine restent des barrières distinctes. Un exécutable compilé n'est pas un livrable de production signé.

## Référence code

- `rust/pravda-server/src/harness_metrics.rs` : `Metrics`, `GateLimits`, `BuildError`, `validate_build`, agrégats.
- `rust/pravda-server/src/bin/pravda-harness.rs` : Tokio, `std::process::Command`, SHA-256, timeouts et orchestration.
- `.github/workflows/rust.yml` : tests Windows/Linux, fixtures intégrées et publication de l'exécutable test x64.
