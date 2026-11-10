# T-004 — Orchestration multi-agents Rust, contrats, sécurité et gouvernance

**Statut : architecture candidate / réalisation incrémentale, non certification.**
**Périmètre : application Windows native Rust + Tauri 2 + WebView2 ; usage privé ; paper-only.**
**Suivi :** [T-004 #186](https://github.com/suvorovpravda/Pravda-Bot/issues/186), [T-003 #184](https://github.com/suvorovpravda/Pravda-Bot/issues/184), [T-002 #181](https://github.com/suvorovpravda/Pravda-Bot/issues/181), [NO-GO #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160).

## 1. Décisions d'architecture

| Proposition initiale | Décision pour Pravda-Bot | Justification |
| --- | --- | --- |
| Grand Maître Orchestrateur (GMO) | Module d'analyse optionnel et isolé ; propose des **hypothèses**, jamais une autorisation d'ordre | Une sortie IA n'est pas une décision de sécurité |
| Risk Engine Guardian (REG) | Moteur **Rust déterministe**, source unique des refus | Veto absolu sur un candidat non conforme |
| Security Sentinel (SS) | Moniteur local de signaux d'incident, sans privilège de réarmement | Le dispositif de coupure doit être indépendant de l'IA |
| Market Data Analyst (MDA) | Validation Rust des flux et des horodatages avant toute analyse | Provenance/qualité/âge explicitement contrôlés |
| User Communication Agent (UCA) | Présentation d'une synthèse traçable sans secrets ni ordres directs | Distinction entre analyse et exécution |
| Incident Responder (IR) | Gestion d'incident locale, rapport expurgé et reprise après approbation | Ne jamais relancer automatiquement un ordre incertain |
| Architect Agent (ARCH) | Contrôle CI, versions, schémas, permissions et provenance | Aucun agent ne peut élargir ses droits |
| Runtime C# / .NET / Python / ccxt / Freqtrade | **Non retenu** dans le runtime | Incompatible avec l'exigence 100 % Rust et packaging Windows existant |
| Bus ZeroMQ local 127.0.0.1 | **Non retenu** ; canaux typés Rust internes et commandes Tauri authentifiées | Préserver l'absence de listener réseau local pour l'application desktop |
| Appels d'API d'exchange depuis GMO | **Interdits** | Seules des interfaces de données autorisées, en lecture seule, passent par les adaptateurs Rust |
| Cloud et modèles alternatifs | Opt-in explicite, budgets stricts, pas de bascule vers réseau hors consentement | Hors réseau = aucune connexion implicite |
| Journal signé par le modèle | **Non** : signer/vérifier dans le code natif avec clés protégées | Le modèle ne détient jamais les clés de signature |
| « Transparence totale de la chaîne de pensée » | **Rationale vérifiable** : entrées, facteurs, hypothèses, risques, alternatives et sources | Pas de trace de raisonnement privé du modèle |
| « Zéro faux négatif » et latence toujours <50 ms | **Objectifs à mesurer**, jamais garantie de sécurité absolue | Démontrer distributions de latence et limites du détecteur |
| AES-256-GCM du prompt | Optionnel pour confidentialité en stockage ; intégrité par hash/version | Le chiffrement ne dissimule pas durablement un prompt embarqué à son utilisateur |

**MCP :** un message JSON n'est pas à lui seul un serveur/client conforme au *Model Context Protocol*. Les contrats ci-dessous sont des contrats métier internes ; une intégration MCP complète serait un adaptateur séparé à spécifier et tester.

## 2. Graphe de confiance

    Sources publiques et APIs lecture seule autorisées
                          │
                         MDA  ──> données/provenance validées
                          │
                   CALCUL RUST (features)
                          │
                          ▼
             GMO / modèle optionnel non fiable
                          │
               proposition / rationale
                          │
                          ▼
                 REG RUST DÉTERMINISTE  <── SS (anomalies)
                   │           │                    │
                 REFUS       PAPER OK            KILL SWITCH
                   │           │                    │
                   └────┬──────┘────────────────────┘
                        ▼
                 UCA / interface Tauri
                        │
                     AUDIT RUST
                        │
                   IR / ARCH (support)
     
    Aucun accès à un adaptateur d'ordre live dans ce lot.

### Autorité et dégradations

- **MDA :** source inconnue, timestamp inconnu, candle OHLCV invalide, données contradictoires ou non fraîches => anomalie et aucune confiance pour la décision ; ne pas fabriquer de valeurs.
- **GMO :** peut fournir une proposition sur données qualifiées ; refuse/hésite si champs requis manquants ; aucune capacité d'écrire des limites, d'accéder aux secrets, ou d'exécuter.
- **REG :** verdict sur données **indépendamment revalidées**, pas à partir du seul score GMO. Si une contrainte obligatoire manque ou n'a pas de modèle fiable => **ESCALATED/REJECTED** selon contrat du consommateur ; jamais ordre live. Rejet terminal pour la requête ; nouvelle proposition seulement sur nouvelle demande authentifiée.
- **SS :** peut **activer** la coupure sur détection critique, mais ne peut pas la désactiver. Un faux positif peut bloquer un système, donc alertes bornées et preuves.
- **IR :** n'exécute aucun ordre/retry ; les états inconnus restent suspendus jusqu'à rapprochement explicite.
- **ARCH :** vérifie version/signatures/tests/reports ; ne peut rendre un release candidat « GO » sur sa propre déclaration.
- **UCA :** ne peut inventer PnL, win rate ou drawdown. Valeurs absentes affichées « non disponible ».

## 3. Contrat d'entrée, de décision et d'audit (v1)

Le code Rust \`crates/pravda-core/src/agent_contract.rs\` introduit un **parseur isolé** d'enveloppe GMO (non branché à une API, un modèle ou un moteur d'ordres). Son entrée est non fiable. Il valide les enums, les champs inattendus et les modes et refuse un objet \`decision\` pour les autres statuts.

**Conventions imposées :**

- \`schema_version: 1\`, \`request_id\` UUID v4, \`status\` dans DECISION / NO_ACTION / INSUFFICIENT_DATA / ERROR ; \`mode\` = PAPER uniquement.
- Les recommandations sont des **propositions de simulation**, non des ordres ; n'utiliser que \`PaperProposal\` avec identifiant d'instrument connu.
- La date de l'analyse, les paramètres de modèle, les latences, le nombre de tokens et les indicateurs doivent venir de systèmes qui les **mesurent réellement**, jamais du modèle lui-même.
- \`rationale\` décrit des raisons brèves, non une chaîne de pensée ni une instruction ; toute phrase tirée d'un contenu externe est traitée comme une donnée.
- Un refus d'analyse, un champ requis manquant ou un type inconnu arrête le workflow ; pas d'exécution par défaut.
- Aucun texte de source (news, social, whale alert) ne peut se faire passer pour une règle de configuration.
- Pas de clés/tokens/PFX, données personnelles ou identifiants clients dans les prompts ni la journalisation des réponses.
- La signature d'un message est un **post-traitement Rust après validation**, jamais une chaîne libre fournie par l'IA.

Le format initial user « gpt-4-turbo | qwen-max » est traité comme exemple historique : choisir le modèle par configuration explicite, sans supposer sa disponibilité, son tarif ou les paramètres de décodage supportés. Une réponse JSON valide ne signifie ni recommandation correcte ni message cryptographiquement signé.

## 4. Politique de risque proposée : R001–R012

Les valeurs ci-dessous proviennent du cahier des charges fourni. Ce sont des **limites maximales candidates pour une future simulation**, pas une permission d'activer le trading réel, ni une validation qu'elles sont adaptées au titulaire.

| ID | Contrôle | Proposition | Refus si |
| --- | --- | --- | --- |
| R001 | Stop obligatoire | stop défini, de bon sens, côté opposé au trade | absent / mauvais côté / illiquide |
| R002 | Perte par opération / equity | 1 % max | calcul indisponible ou dépassement |
| R003 | Risk/Reward minimal | 2.0 | calcul de payoff manquant / plus faible |
| R004 | Montant de position | valeur approuvée par profil | seuil absent ou dépassé |
| R005 | Levier brut | 3× maximum envisagé | levier inconnu/supérieur ; marge inconnue |
| R006 | Drawdown journalier projeté | 3 % max | atteint / état inconnu |
| R007 | Drawdown hebdomadaire projeté | 7 % max | atteint / état inconnu |
| R008 | Exposition sectorielle | 25 % equity max | secteur manquant ou dépassement |
| R009 | Corrélation des positions | 0,7 max | mesures manquantes / incertaines |
| R010 | Coussin de marge | 20 % min du besoin calculé | collatéral insuffisant |
| R011 | Concentration par actif | 40 % equity max | dépassement |
| R012 | Réarmement après arrêt | délai minimal 5 min + action **humaine explicite** | kill switch actif, état non rapproché, consentement absent |

Les seuils ne sont **pas** codés en dur dans l'application par cette ADR. Les stratégies autorisées (TREND_FOLLOWING, MEAN_REVERSION, BREAKOUT, FUNDING_RATE_ARB, LIQUIDATION_CASCADE, SENTIMENT_DIVERGENCE) constituent une **liste de candidates à valider par backtest indépendant**, non une preuve de rentabilité. Le risque calculé depuis un stop-loss n'est pas une perte garantie.

## 5. Entrées et anomalies de marché

| Catégorie | Vérification avant GMO | Réaction |
| --- | --- | --- |
| OHLCV | OHLC cohérent, temps monotone, unité de volume, continuité attendue | REJECT_DATA sur incohérence |
| Order book | meilleur bid inférieur/égal au meilleur ask, profondeur, spread, source et fraîcheur | INSUFFICIENT_DATA, aucun ordre |
| Cross-exchange | comparabilité réelle (instrument, devise, type, timestamp, frais) | Signaler divergence ; ne jamais ignorer un marché particulier sur seuil arbitraire |
| Funding / OI | période, contrat, timestamp et unité obligatoires | Rejeter données incompatibles |
| Flux sentiment/news | provenance et horodatage ; textes non fiables | Jamais de commandes exécutables |
| Flash crash / depeg | changement extrême détecté par mesures éprouvées, non estimation IA | Pause / demande d'examen |
| Latence | âge de chaque flux séparément ; SLA 500 ms uniquement si sensé pour la source | Données inconnues/stale plutôt que réutilisation |
| Liquidation et slippage | coûts et scénarios adverse obligatoires pour simulation dérivés | Refus si risque non modélisé |

La qualité de données n'est pas une autorisation implicite de position. Même des données valides peuvent contenir manipulation ou erreurs non détectées.

## 6. Audit, secrets et exploitation

1. Chaque événement garde \`request_id\`, version de schéma, catégorie d'événement, timestamp source/ingestion, hash de la configuration, état de la décision, métriques effectivement mesurées et raison de refus expurgée.
2. Chaînage de hash et signatures natifs, clés protégées et rotation documentée, vérification au démarrage et à l'export. Un fichier local append-only est **tamper-evident**, pas techniquement immuable à lui seul.
3. Signature de messages aux **frontières de processus ou de transport** ; si tous les agents résident dans le même processus, préférer types Rust et séparation des capacités ; aucune clé dans le prompt.
4. Stockage de secrets sous Windows Credential Manager/DPAPI selon fonctionnalités validées. Réseau externe uniquement par adaptateurs allowlist et autorisation consciente.
5. Coupure immédiate fail-closed en cas de menace : désactivation des nouveaux candidats, journal et alerte UI. **Pas d'arrêt sans preuve de reprise**, pas de retry aveugle.
6. Réarmement après approbation humaine liée à la session et contrôle du portefeuille/données/règles ; **jamais uniquement après expiration d'un minuteur**.
7. Mode hors réseau : services cloud et connecteurs désactivés et aucun appel implicite ; le modèle local demeure optionnel et dépend de ressources matérielles vérifiées.
8. Updater manuel et signé, diagnostics et rapports de performance sur SHA/artefact exact, aucun contournement SmartScreen.

## 7. Phasage et tests d'acceptation

| Lot | Livrable | Exigence d'acceptation |
| --- | --- | --- |
| T-004.A | ADR + enveloppe de réponse Rust strictement validée | Tests unitaires positifs/négatifs ; JSON inconnu refusé |
| T-004.B | MDA Rust | Fixtures OHLCV, stale, volume, tampering, confusion de symbole/devise |
| T-004.C | REG Rust | R001–R012, refuss déterministes et stress, critères mesurables |
| T-004.D | SS / kill switch | État persisté, stop immédiat, aucun rearm automatique ; redémarrage |
| T-004.E | GMO optionnel | Sortie qualifiée, provenance, prompt injection, coût plafonné, aucun accès broker |
| T-004.F | UCA / IR / audit | Données absentes affichées, redaction, export vérifiable et postmortem |
| T-004.G | GUI, packaging, tests sécurité | Régression IPC, tests Win/WebView2, tests installés MSI/NSIS, RustSec, signature |

**État vérifié de cette livraison documentaire :** la présence de contrats et de tests source ne prouve pas encore une exécution. **Toute nouvelle fonctionnalité prend l'état PLANNED ou TESTING jusqu'aux preuves exactes au SHA.**

**Livraison Windows :** [#106](https://github.com/suvorovpravda/Pravda-Bot/issues/106) et [#160](https://github.com/suvorovpravda/Pravda-Bot/issues/160) conservent leur autorité ; **NO-GO**, aucune permission d'exécution réelle ajoutée.

⚠️ Ceci est une analyse algorithmique, pas un conseil financier.
