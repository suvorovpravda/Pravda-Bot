# ADR-001 — Périmètre trading et frontières d'autorisation (T-001)

**Statut : proposition de contrat fonctionnel, non homologuée.**  
**Référence :** [Issue #179](https://github.com/suvorovpravda/Pravda-Bot/issues/179) · [Plan de livraison #106](https://github.com/suvorovpravda/Pravda-Bot/issues/106) · [Sécurité / GO-NO-GO #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160)  
**Référence du code inspecté :** `main@b602e7c08212fca5505bab5060f88e202584ea98` (10 novembre 2026).

## 1. Décisions et exclusions explicites

| Domaine | Autorisé en phase actuelle | Non autorisé par cette ADR |
| --- | --- | --- |
| Données | Marchés publics et données de comptes en lecture seule via interfaces officielles et autorisées | Contournement d'API, collecte de secrets ou d'informations sans permission |
| Analyse | Recherche, classement, simulation et propositions explicables | Promesse de rentabilité, conseil personnalisé non validé |
| Allocation | Hypothèse configurable à étudier : 50 % crypto / 50 % actions ; simulation avec cash, frais et conversion de devises explicités | Règle d'allocation automatique imposée ou réinvestissement réel |
| Horizons | Court, moyen et long termes en simulation, paramètres définis par stratégie | Renversement automatique de position ou position opposée réelle |
| Instruments | Long, short, futures, options ou autres dérivés **uniquement si modélisés et explicitement autorisés pour la simulation** | Tout produit non reconnu, risque non calculable, levier ou opération réelle |
| Mode réel | Aucune transmission d'ordre ; configuration effective `live_enabled = false` | Activation par stratégie, interface, déploiement, mise à jour ou IA |
| Distribution | Application Windows native Rust/Tauri 2 et composants actuellement validés | Revendiquer une homologation FR/CH, une signature ou un GO production sans preuves |

**Le code courant n'implémente pas ce contrat intégralement.** En particulier, `crates/pravda-engine/src/research_risk.rs` est une fonction pure de recherche papier, non reliée au simulateur actif ni à un OMS. Sa réussite ne prouve ni le contrôle des shorts/dérivés ni une autorisation de trading réel.

### Modes d'exploitation distincts

1. **PERSONAL_RESEARCH** — utilisateur analysant ses propres données et simulant des décisions ; pas de mouvements réels.
2. **SOFTWARE_DISTRIBUTION** — logiciel fourni à des utilisateurs gérant leurs propres comptes ; fonctionnalités, pays de distribution et obligations à vérifier avant commercialisation.
3. **THIRD_PARTY_ADVICE_OR_MANAGEMENT** — conseil individualisé, gestion discrétionnaire, conservation ou passage d'ordres pour des tiers : **interdit dans le lot actuel** ; exiger une analyse réglementaire propre à chaque service, entité, client et juridiction.

L'accès à un courtier ou à un exchange, l'existence d'un compte, une offre payante d'IA ou la déclaration d'un utilisateur ne confèrent **aucune** autorisation réglementaire.

## 2. Frontière de confiance

```text
Flux publics / comptes autorisés en lecture seule
                     │
                     ▼
          Vérification provenance / âge
                     │
                     ▼
            Analyse et stratégies IA
                     │
            Proposition non fiable
                     ▼
        Risk Gate central côté Rust
          ├─ mode effectif : PAPER
          ├─ instrument et données connus
          ├─ limites / perte / exposition
          ├─ marge / collatéral / stress
          └─ coupe-circuit et journal
                     │
            ┌────────┴────────┐
            ▼                 ▼
          REFUS        SIMULATION LOCALE
                              │
                        Aucun ordre live
```

L'IA produit une proposition, jamais un privilège : elle ne modifie pas les limites, la liste blanche, le mode du moteur, les paramètres de conformité ou la capacité IPC. Les validations doivent être exécutées côté Rust sur chaque demande ; aucun bouton ou drapeau côté WebView ne suffit.

## 3. Contrat de données et règles fail-closed pour futures implémentations

Tout candidat de simulation de short, dérivé ou levier doit porter une identité d'instrument explicite et versionnée, sa juridiction applicable, son sens (achat/vente), le type (spot, short avec emprunt, future, option, autre), la devise, la quantité et une provenance des prix horodatée. Pour les dérivés, ajouter au minimum multiplicateur, échéance si applicable, mode de règlement et coût de financement ; pour la vente à découvert avec emprunt, disponibilité, conditions, coût et fraîcheur de l'emprunt. Un champ obligatoire inconnu ou absent entraîne **REFUS**.

Pour chaque proposition, calculer avec des montants à précision contrôlée, sans mélange d'unités :

- Notionnel, exposition brute et nette, exposition par marché/actif/instrument/stratégie et concentration des positions corrélées.
- Collatéral disponible, marge initiale et de maintenance, coussin après frais et variation extrême ; ne jamais considérer la liquidation automatique ou un stop-loss comme une borne garantie de perte.
- Pertes réalisées et latentes, cumul journalier/hebdomadaire, frais, funding, slippage, taux de change, gaps de prix et scénarios de stress.
- Restriction la plus forte entre réglementation applicable, contraintes de courtier et politique utilisateur approuvée ; toutes les limites indispensables doivent exister, être valides et testées. **Aucune valeur chiffrée inventée** ne vaut politique approuvée.

**Invariants :**

- `live_enabled = false` en absence de preuve d'autorisation indépendante ; en mode réel non qualifié, exposition et levier permis = **0**.
- Un paramètre absent, une donnée périmée, une unité incompatible, un calcul non borné, une expiration inconnue, une limite incohérente, une marge insuffisante ou une incertitude de statut d'ordre ⇒ **REFUS**.
- Coupe-circuit central déclenché par perte excessive, désynchronisation portefeuille/venue, panne réseau/API, liquidité insuffisante, prix non fiables ou marge insuffisante ⇒ aucun nouvel ordre. Un statut d'ordre inconnu nécessite rapprochement **avant** reprise ; jamais de nouvel essai aveugle.
- Les stratégies, modèles d'IA, UI et modules optionnels ne peuvent pas dégrader ces invariants. Garder un journal auditable des règles et décisions **sans clés, tokens ni données sensibles inutiles**.
- Les résultats d'un backtest ne sont ni des prix de marché live ni des garanties de performance ; inclure les frais, les situations de marché extrêmes et les biais de simulation.

## 4. Décisions laissées ouvertes (aucune valeur présumée)

| Code | Décision nécessaire avant implémentation live | Autorité requise |
| --- | --- | --- |
| D-01 | Usage personnel, distribution logicielle ou service pour comptes tiers | Responsable du produit + revue juridique |
| D-02 | Entité exploitante, clientèle, résidence, services réglementés, pays et autorisations vérifiables | Conseil juridique/compliance FR-CH |
| D-03 | Allocation 50/50 : portefeuille total ou poche investie ; traitement cash, rééquilibrage, gains, frais et impôts | Politique d'investissement validée |
| D-04 | Instruments admissibles (short cash, futures, options, CFD...), venues, disponibilité, permissions client | Compliance + venue + responsable risque |
| D-05 | Seuils d'exposition, levier, perte, marge, liquidité, délai de fraîcheur, stress et kill-switch | Responsable risque, tests reproduisibles |
| D-06 | Procédure d'approbation humaine, audit, reprise, rapprochement d'ordres et désactivation d'urgence | Responsable sécurité + opérations |

Tant que ces décisions ne sont pas documentées et approuvées, ne renseigner **aucun plafond live positif** et ne déclarer aucune conformité acquise.

## 5. Matrice d'acceptation à implémenter et tester

| ID | Scénario | Comportement attendu |
| --- | --- | --- |
| T001-01 | Candidate paper autorisée, prix et règles valides | Décision déterministe, simulation uniquement |
| T001-02 | Demande live depuis stratégie, IA ou UI | Refus avant tout appel réseau de passage d'ordre |
| T001-03 | Instrument ou multiplicateur inconnu | Refus, motif borné et auditable |
| T001-04 | Short sans emprunt confirmé ou informations d'emprunt périmées | Refus |
| T001-05 | Marge insuffisante / stress de gap / dépassement du levier | Refus et événement de coupe-circuit selon politique |
| T001-06 | Pertes journalières / hebdomadaires ou exposition après opération excessives | Refus sans exception par stratégie |
| T001-07 | Prix périmé, API indisponible, horloge incohérente ou montant qui déborde | Refus ; aucune valeur de remplacement silencieuse |
| T001-08 | Ordre au statut inconnu après panne | Aucun nouvel essai ; état à rapprocher |
| T001-09 | Compte, instrument, client ou pays non autorisé | Refus ; aucun contournement par configuration UI |
| T001-10 | Test sur branche ou SHA ancien | Ne constitue jamais une validation du code ou binaire final |

**État des preuves au 10/11/2026 :** cette matrice est une **spécification**, pas un relevé de tests exécutés. Aucun des scénarios nouveaux ne peut être marqué PASS sans implémentation et résultats frais sur le SHA pertinent.

## 6. Étapes suivantes et conditions de livraison

- **T-002 :** modèle de menaces spécifique short, marge, liquidation, API d'ordres, positions opposées, IA et manipulation de marché ; tests négatifs.
- **Implémentation ultérieure :** moteur Rust strictement papier, contrats typés à unités sûres, tests unitaires de limites et intégration avec le simulateur existant ; zéro effet de bord externe.
- **Lot live futur et séparé :** sécurité, permissions, éligibilité, réglementation, confirmation humaine, contrôles d'ordres, reconciliation, résilience, audit indépendant, preuve CI/Windows au même SHA.
- **Livraison Windows 30/11/2026 :** ne pas élargir le chemin critique sans décision explicite ; les conditions de `#106` et `#160` restent **NO-GO** jusqu'à preuve réelle.

Cette ADR spécifie le futur périmètre mais n'active ni trading réel, ni conseil, ni gestion pour tiers, ni updater automatique. Aucun versement ou mouvement de fonds n'est requis pour valider T-001.
