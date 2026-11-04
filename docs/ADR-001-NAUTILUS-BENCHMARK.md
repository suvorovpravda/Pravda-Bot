# ADR-001 — Référence NautilusTrader : évolution sécurisée du moteur Pravda-Bot

**Statut : PROPOSÉ — exigences du client final, pas fonctionnalités livrées**  
**Date : 4 novembre 2026**  
**Décision :** réutiliser les idées d'architecture de NautilusTrader en **Rust natif**, sans intégrer directement son code tant que l'audit technique/juridique n'est pas terminé. Aucun trading réel autorisé.

Sources examinées : [NautilusTrader](https://nautilustrader.io/), [dépôt develop au commit a5c5b1d](https://github.com/nautechsystems/nautilus_trader/tree/a5c5b1d02b74e32d20f7dec1ed467270be3fe20a), [architecture](https://nautilustrader.io/docs/latest/concepts/architecture/), [event sourcing](https://nautilustrader.io/docs/latest/concepts/event_sourcing/), [DST](https://nautilustrader.io/docs/latest/concepts/dst/), [modèles comportementaux](https://nautilustrader.io/docs/latest/concepts/behavioral_models/), [types monétaires](https://nautilustrader.io/docs/latest/concepts/value_types/), [live vs backtest](https://nautilustrader.io/docs/latest/concepts/live/), [licence LGPL-3.0](https://github.com/nautechsystems/nautilus_trader/blob/develop/LICENSE).

**Cadrage :** NautilusTrader est un framework/moteur de trading événementiel Rust, avec composantes et bindings Python ; le site propose aussi des offres commerciales distinctes. Il ne s'agit **pas** de remplacer l'EXE Tauri de Pravda-Bot par une application Nautilus complète ni d'installer Python chez l'utilisateur. Le dossier « Crypto-Hunter v3 » est une autre demande fonctionnelle issue du financeur : [#121](https://github.com/suvorovpravda/Pravda-Bot/issues/121). Le **plan de sécurité et livraison Windows #106** demeure prioritaire.

## Décision et bénéfices attendus

Conserver l'application Windows autonome Tauri 2, les modules `pravda-core`, `pravda-engine`, `pravda-fetchers`, `pravda-store`, le service `DesktopRuntime` et l'IPC Rust protégé ; ajouter une **couche d'événements, de risque et de recherche reproductible**. L'interface et les fonctions DeFi/on-chain deviennent des **consommateurs de données**, jamais des détenteurs de secrets ni des émetteurs d'ordres.

### Matrice d'écarts vérifiée sur Pravda-Bot

| Idée issue de NautilusTrader | État observé dans Pravda-Bot le 04/11 | Amélioration précise | Lot |
| --- | --- | --- | --- |
| Noyau unifié de stratégie / bus | Rust core existe ; diffusion ticks Tokio, pas de contrat de causalité d'événements partagé | Message envelope typée, bus borné, idempotence, trace correlation/causation, ordre total par flux | P0 architecture |
| Horloges d'exécution | Horloges `Instant` et `Utc`, replay déterministe limité | Séparer exchange timestamp, observed/received timestamp, monotonic clock, replay logical time ; tester désordre/retard | P0 |
| `Price`, `Quantity`, `Money` fixes | `f64` pour cotations/simulation et certains calculs ; validation NaN | Types distincts à précision fixe, devise/asset explicite, arrondis et taille de lot par venue ; migration sans régression | P0 pour tout futur ordre |
| Event sourcing durable | SQLite WAL, ticks et journal d'ordres existent, sans event-store général | Événements *state-affecting* append-only, seq, run_id, schema_version, checksum/intégrité, rejouabilité + snapshots | P0 |
| État reconstruisible | Snapshots et journal partiels, WAL testé | Cache comme projection, restauration après crash, divergence détectée et blocage plutôt que faux PASS | P0 |
| Modèles de simulation | PaperSimulator et `backtest.rs` hors ligne ; frais, cash et seuils | Modèles interchangeables de frais maker/taker, slippage, latence, fill/partial-fill, carnet, liquidité et coûts de financement | P1 |
| Évaluation de stratégie | Tests/replay mais pas de métriques de risque exhaustives | Max drawdown, Sharpe/Sortino *si admissibles*, turnover, coût/slippage, limite de pertes, coût de données, biais et couverture | P1 |
| Risk engine central | Contrôles locaux divers, ordre réel non homologué | Garde-fous indépendants et fail-closed : exposition, taille, concentration, levier, liquidité, prix stale, pertes, limite d'opérations, kill switch logiciel | P0 |
| OMS + réconciliation | Journal d'ordres/contrats Revolut expérimentaux | États explicites `NEW/PENDING/ACK/PARTIAL/FILLED/CANCELED/REJECTED/UNKNOWN`, client_id idempotent, zéro retry aveugle, rapprochement read-only | P0 avant live |
| Adaptateurs multi-venue | Kraken/Binance prix ; Revolut X adapté partiellement ; Coinbase absent | `VenueId` et `InstrumentId` explicites ; déclarer capacités, contraintes, taux, alimentation et fraîcheur | P1 |
| Orderbook + OHLCV | Cotations/ticks, pas de carnet multi-venue complet prouvé | Carnets L2 si fournisseur/légalité/qualité le permettent, bougies OHLCV avec closes vérifiées | P1 |
| Moteur DeFi / on-chain | Aucun adaptateur on-chain/DeFi dans `pravda-fetchers` | Adapter observations whale/stablecoins/netflows/DeFi de #121 comme `CustomData` avec provenance et risque | P1 |
| Test de simulation déterministe (DST) | Replays offline déterministes partiels ; pas d'ordonnanceur DST global | Seeds, horloge simulée, gigue/timeout et événements intercalés, crash/restart, tests de propriétés et traçage d'une seed défaillante | P0/P1 |
| Catalogue historique | SQLite WAL simple pour ticks | Parquet/Arrow **optionnels** pour jeux massifs de recherche, jamais source d'autorité des comptes/ordres ; manifest data/sha | P2 |
| Interface de suivi et audit | Tauri UI, Doctor, Harness, centre de sécurité #120 en cours | Panneaux Risk/Portfolio/Backtesting/On-Chain, source+qualité+UTC, rapports d'événements, état en temps réel honnête | P1 |
| IA/ML | LM Studio et idées LLM anciennes, aucune preuve de modèle profitable | Orchestrateur de recherche facultatif **hors décisions live** ; aucune exécution de code tiers dans la WebView | P2 |
| Research Python | Pas de Python requis pour la distribution desktop | Environnement de recherche **séparé** facultatif pour notebooks, pas de dépendance runtime chez le client Windows | P2 |
| Quantités multi-actifs | Projet orienté crypto BTC/ETH/SOL | Contrat extensible futures/options/FX seulement si données, dérivés, marges et réglementation validés ; aucune promesse de couverture immédiate | P2 |

## Invariants fondamentaux : ce qui ne doit JAMAIS changer

- Mode **trading réel OFF** tant que #89 et les barrières #106 ne sont pas homologués ; aucun ordre envoyé par Doctor, Harness, CI, centre de sécurité ou module on-chain.
- Tauri IPC `main` seulement, session Rust vérifiée ; WebView ne porte aucun secret et n'invoque pas de shell, Node, Python ou serveur HTTP local.
- Les trois livrables Windows séparés (MSI/NSIS/app, Harness, Doctor) restent autonomes ; composants optionnels sélectionnables lorsque la sélection est réellement implémentée, **sécurité centrale indésactivable**.
- En cas de perte de réseau, d'horloge incohérente, de base corrompue ou d'exécution incertaine : **STOP/UNKNOWN explicite**, jamais émission ou retry opportuniste.
- Pour les futures intégrations : pas de réutilisation silencieuse de code LGPL-3.0 ; décider entre **implémentation originale de concepts** et **dépendance Nautilus** après vérification des obligations LGPL (avis, offre de source/relinking selon mode de liaison, versions, licences transitives, conformité distribution MSI/NSIS) et analyse de la supply chain.

## Contrats proposés

```text
EventEnvelope {
  schema_version, run_id, seq, event_id, correlation_id, causation_id,
  category, venue_id?, instrument_id?, source_id,
  event_time_utc_ns, received_time_utc_ns, logical_time_ns?,
  payload_type, payload_digest, provenance, quality, replayable
}
RiskAssessment {
  request_id, policy_version, instrument_id, venue_id,
  allowed = false_by_default, reason_codes[], checked_at,
  exposure_before, exposure_after, data_age_ms, state_version
}
ExecutionIntent {
  client_order_id, instrument_id, venue_id, side, quantity_fixed,
  order_type, time_in_force, strategy_id, session_id
}
ExecutionOutcome { client_order_id, status, venue_order_id?, venue_report_id?, reconciled_at? }
ResearchRun {
  run_id, engine_commit, input_manifest_sha256, strategy_version,
  risk_policy_version, simulation_seed, fee_fill_latency_models,
  start_utc, end_utc, observed_at, result_digest
}
```

**Contrat de temps** : les timestamps en nanosecondes *représentables* n'impliquent pas que Windows ou les API fournissent réellement une précision d'une nanoseconde. Les horloges simulées et réelles ont des garanties distinctes. Définir explicitement l'ordre de traitement pour observations simultanées et événements rétractés, et n'utiliser aucune donnée future.

**Contrat d'exécution** : `submit` peut finir en état `UNKNOWN` à cause du réseau. Rechercher l'ordre via identifiant durable/réconciliation avant toute nouvelle action. Un identifiant client unique n'implique pas « exactement une fois » sur tous les fournisseurs. Les règles de venues prédominent. Les transactions externes et les confirmations différées sont prises en charge dans le modèle *mais non autorisées à la production*.

**Contrat calcul** : conversions décimales contrôlées pour Price, Quantity, Money ; rejet overflow/underflow, instruments fantômes, rounding silencieux, précision non conforme, fees non-finies ; ne jamais réutiliser les `f64` de simulation comme preuve de précision financière.

## Interaction avec le projet Crypto-Hunter / On-Chain #121

On-chain est une **famille de données hétérogènes** avec blockchain, txid/log_index, confirmations, provenance de labels et temps d'observation. Ces données suivent les mêmes `EventEnvelope` / filtres qualité que cotations et carnets, mais les mesures **ne transitent pas par le moteur d'ordres**. Netflows de plateformes, USDT/USDC/DAI, transferts whales, TVL/DEX et RSI/MACD/Bollinger produisent des *observations* et *alertes à expliquer*. Le score d'activité [0,100] est **non directionnel** ; le risque et la confiance sont affichés séparément. Le retour « marché actif » ne signifie jamais « acheter ».

Les fournisseurs externes ont une couverture et des révisions de données : exclure toute recommandation lorsque le label de wallet est incertain, que le provider est hors quota ou que les séries sont périmées. Les contrats sont versionnés et chaque replay utilise **l'information disponible à cette date**, pas une version future d'une attribution exchange.

## Plan d'implémentation, avec jalons et portes

1. **Socle de sécurité P0 avant toute extension live** (#106/#120) : finaliser GUI sécurité, authentification, import/export correct, sécurité installateur et signature selon label. Ne pas ouvrir davantage de PR majeures simultanées.
2. **Contrats Rust et essais déterministes** : petites PR : types `Price/Quantity/Money` versionnés, `InstrumentId`, enveloppe d'événement, politique de risque pur/append-only ; zéro changement de transport ; tests unitaires/proptest.
3. **Journal d'événements transactionnel** : migration SQLite additive avec idempotence/transaction et index, événement/correlation/seq ; snapshots cohérents ; rejeu après crash avec preuve de même résultat, aucune exposition de secret.
4. **Simulateur execution** : module `ExecutionModel` injectable : modèle de fill, frais, maker/taker, latence, liquidité et annulations, contenu dans Harness/backtest. Métriques de faux positifs, drift, drawdown ; seed de replay. **Aucun appel broker.**
5. **Moteur de risque** : décider/rejeter avant création d'un *paper intent* ; table des raisons dans centre de sécurité, tests d'épuisement de risques, ordre inconnu, blocage d'ordres en cas de données manquantes.
6. **Recherche on-chain / DeFi** : selon #121, providers approuvés, schéma et qualité, flux read-only, aucun package JavaScript externe ; score d'activité explicable.
7. **Multi-venue et interface** : capacités des adaptateurs, carnet/portfolio/rapports si sources vérifiées ; Tauri onglet Intelligence + alertes + provenance, unifiée pour Windows.
8. **Qualification** : soumettre modèles à la recette Windows sur SHA exact (E2E IPC, résilience hors ligne, CodeQL, signatures/commits, fixtures horodatées, crash, 429, latence, data corruption). Publication selon décision GO/NO-GO.

**Hors périmètre du 30 novembre sans dérogation explicite accompagnée de preuves :** identité NautilusTrader complète, exécution multi-venue réelle, futures/options avec marges, performance HFT ou latence garantie, moteur Python/Redis embarqué, IA qui négocie, service cloud, plan payant Nautilus, ou intégration de code LGPL. Le chantier ne doit pas faire passer un livrable P0 validé en NO-GO par ajout de fonctions non qualifiées.

## Tests de recette obligatoires

- [ ] event logs monotones et déterministes, rejouabilité depuis snapshots, gestion séquences sautées/dupliquées et corruptions
- [ ] race de shutdown/redémarrage, seed replay et interleavings différents, aucun appel externe en mode Harness isolé
- [ ] même logique Rust de signal en simulation et en production `read-only`, mais **aucune identité artificielle** des conditions d'exécution et fills
- [ ] types monétaires précis, erreurs de décimales, quantités négatives, overflow, devises et instruments incompatibles
- [ ] risk engine fail-closed, limites exposées et données obsolètes, aucune permission d'ordre live depuis WebView
- [ ] modèles de frais/fill/latence comparés à fixtures, partial fills et pertes maximales ; métriques avec provenance
- [ ] documents de licence et SBOM si une dépendance Nautilus est finalement intégrée
- [ ] conformité à #106, #120, #121 ; toute protection manquante = NO-GO pour la fonctionnalité concernée

## Sources et versionnement

- Version examinée : `nautechsystems/nautilus_trader` branche `develop`, commit **a5c5b1d02b74e32d20f7dec1ed467270be3fe20a** au 04/11/2026 ; la documentation `latest` peut évoluer sans notification.
- LGPL-3.0 ne signifie ni intégration automatique sûre ni droit d'ignorer les obligations de distribution. Les offres commerciales `Pro/Cloud/Institutional` sont distinctes du dépôt open-source.
- Références : liens en tête du document ; garder le SHA de la source d'inspiration dans toute revue de compatibilité et les fichiers de preuves.

**Résumé décisionnel :** retenir *l'architecture événementielle, les types monétaires précis, le journal durable, la simulation réaliste, le risque centralisé et les tests déterministes* ; reporter l'intégration directe du projet NautilusTrader, le code Python en production et toutes les connexions de trading réel jusqu'à revue de sécurité et licence.
