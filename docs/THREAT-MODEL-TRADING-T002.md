# T-002 — Modèle de menaces financières et IA (paper-only)

**Statut : THREAT MODEL / conception à valider — aucun GO production.**
**Baseline inspectée : main@b602e7c08212fca5505bab5060f88e202584ea98 (10/11/2026).**
**Suivi :** [Issue #181](https://github.com/suvorovpravda/Pravda-Bot/issues/181) | [T-001 #179](https://github.com/suvorovpravda/Pravda-Bot/issues/179) | [ADR-001 proposée en PR #180](https://github.com/suvorovpravda/Pravda-Bot/pull/180) | [Livraison #106](https://github.com/suvorovpravda/Pravda-Bot/issues/106) | [NO-GO #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160)

## 1. Périmètre et preuves existantes

La présente analyse concerne les **futurs** instruments long, short, dérivés, options, futures, positions opposées et levier. Le périmètre utilisable reste **paper trading / lecture seule**. Les stratégies et propositions de l'IA ne peuvent déclencher aucun ordre réel.

| Élément du dépôt | État connu au SHA inspecté | Conséquence |
| --- | --- | --- |
| crates/pravda-engine/src/research_risk.rs | **Implémenté pour recherche papier seulement :** validation de limites nominales, exposition totale, perte réalisée journalière, prix, âge de cotation, écart et débordement entier ; non câblé au simulateur actif | Son succès n'est pas une autorisation d'ordre, ni un contrôle short/levier |
| crates/pravda-engine/src/backtest.rs | **Implémenté :** replay hors ligne à prix fournis, avec refus des lignes invalides et statut d'approbation simulé ; ne modélise pas liquidation, liquidité, latence ou remplissage réel | Les gains simulés sous-estiment potentiellement les risques |
| Tauri / interface / session IPC | Séparation native attendue selon les contraintes du dépôt ; **aucune vérification E2E nouvelle réalisée par cette ADR** | Les nouveaux paramètres de risque devront être validés côté Rust |
| Connexion Revolut X | Cadrage de référence : lecture seule ; **aucune API d'ordre autorisée par T-002** | Tout live constitue un lot indépendant |
| Autorisations France/Suisse | **Non vérifiées pour des clients ou services spécifiques** | Pas d'activité réglementée présumée permise |

Cette ADR est un livrable de conception ; sa création, la CI verte de documentation ou un bon score de tests de moteur ne constituent **pas** une démonstration d'intégration, de conformité, de sécurité en production ou de disponibilité des exchanges.

## 2. Actifs, acteurs, frontières de confiance

### Actifs à protéger
- Solde, équité, collatéral, positions, limites par compte, juridiction et instrument.
- Exactitude des prix, taux de change, multiplicateurs, coûts, marge et états d'exécution.
- Secrets/API, authentification, journaux d'audit, historique des décisions, données clients.
- Possibilité de conserver le trading réel désactivé sans voie de contournement.

### Sources / acteurs
- **Utilisateur authentifié :** peut proposer une politique de simulation ; ne confère aucun droit réglementaire.
- **Stratégies et modèles IA :** proposent des actions et paramètres ; ils sont **non fiables** pour la sécurité et l'exécution.
- **Données d'exchange, prix et textes externes :** possiblement incomplets, périmés, manipulés ou mal interprétés.
- **Backend Rust :** seule frontière admissible pour vérifier les invariants ; validation à chaque requête.
- **Courtier / exchange :** autorité externe sur ordres, permissions, emprunts, marge et statut réel, pour un éventuel lot futur.
- **Administrateur, chaîne CI/CD, mise à jour :** peuvent changer le code ; signatures et preuve de livraison conditionnent l'autorisation de diffusion.

### Flux de confiance cible

    Données venues / comptes lecture seule / textes et IA (NON FIABLES)
                              |
                              v
                    Normalisation stricte
               source + horodatage + unité + type
                              |
                              v
                   Proposition de stratégie
                              |
                              v
                Gate de risque central RUST
          mode PAPER + limites + instruments + stress
                  /                   \
                 /                     \
              REFUS              SIMULATION LOCALE
                                     |
                             jamais d'API d'ordre

**Interdit :** chemin alternatif stratégie → courtier, WebView → secret/API d'ordre, mise à jour → auto-activation du mode live, IA → altération de limites ou masque de refus. Contrôles visuels et stop-loss ne suffisent pas.

## 3. Registre de menaces

Échelle : P0 = risque de perte excessive, d'ordre non autorisé ou de violation de frontière ; P1 = perte substantielle de fiabilité/conformité ou fraude indirecte. La criticité est **provisoire**, sans scores probabilistes inventés.

| ID | Priorité | Déclencheur / scénario | Risque | Invariant / mitigation exigée |
| --- | --- | --- | --- | --- |
| M-01 | P0 | Une suggestion IA, un flag UI ou une option de démarrage active live | Ordre non autorisé | Mode live OFF en Rust ; zéro adaptateur d'ordre dans le lot actuel |
| M-02 | P0 | Une stratégie contourne le moteur ou modifie ses limites | Contrôle de risque contourné | Gate Rust unique, limites non modifiables depuis la stratégie, contrôle par requête |
| M-03 | P0 | Prix ancien, faux, null, NaN ou source ambiguë | Entrée/prix de liquidation erronés | Refus source/temps/qualité non vérifiés, pas de prix synthétique de remplacement |
| M-04 | P0 | Multiplicateur, taille, devise ou conversion incohérents | Notionnel sous-estimé | Unités typées, conversion explicite, bornes, arrondis prudents et refus en cas d'ambiguïté |
| M-05 | P0 | Short sans emprunt localisable, frais inconnus ou rappel de titres | Rachat forcé / perte et frais inattendus | Disponibilité/fraîcheur emprunt, frais, risque de recall ; sinon refus |
| M-06 | P0 | Futures/perp avec marge initiale/maintenance inconnue | Appel de marge / liquidation | Marge validée selon la venue et stress de gap/volatilité/funding ; sinon refus |
| M-07 | P0 | Options avec exercice/assignation, payoff ou multiplicateur ignoré | Exposition non linéaire / assignation imprévue | Modèle propre à chaque instrument et échéance, risque maximal calculable ou refus |
| M-08 | P0 | Long et short opposés compensés seulement en net | Levier et exposition brute masqués | Deux jambes comptabilisées en brut ; corrélation, basis et échec d'une jambe stressés |
| M-09 | P0 | Un hedge est exécuté partiellement, une seule jambe subsiste | Exposition directionnelle non voulue | États indépendants par jambe, contrôle de risque brut ; suspension et reprise définie |
| M-10 | P0 | Coupure réseau au moment d'un ordre, statut inconnu, retry aveugle | Ordre doublonné | Pour futur live : idempotence et rapprochement confirmé AVANT retry ; sinon blocage |
| M-11 | P0 | Pertes réalisées/latentes et limites quotidiennes incohérentes | Pertes supérieures au budget | Contrôle global pré-trade et post-trade, pertes par fenêtre, coupe-circuit non débrayable |
| M-12 | P0 | Collatéral retiré, changement soudain de marge ou de taux | Liquidation forcée | Snapshot collatéral à jour, haircuts et politique de marge, refus si état incertain |
| M-13 | P1 | Slippage, funding, borrow, spreads et frais non simulés | Rentabilité artificielle | Coûts explicités, fourchettes conservatrices et signalement des hypothèses |
| M-14 | P1 | Données du futur, survivorship bias ou prix mal synchronisés en backtest | Fausses conclusions | Jeux horodatés, tests anti-lookahead, reproductibilité et provenance |
| M-15 | P1 | Texte web/prompt injection commande de changer des règles | Escalade de privilèges IA | Contenu externe = données, sortie IA = proposition, validation indépendante sans secrets |
| M-16 | P0 | Pays ou classification du client incorrects, permission non autorisée | Opération inéligible/illégale | Juridiction et permission vérifiées par services qualifiés ; aucun live sinon |
| M-17 | P1 | Journal, diagnostic, crash-report ou prompt divulgue des credentials | Compromission du compte | Exclusion des secrets, chiffrement du coffre, redaction, contrôle de rétention et export |
| M-18 | P0 | Updater, binaire non signé ou configuration modifiée active live | Escalade persistante | Conditions #160 : signature, provenance, rollback, mode live OFF indépendant de la version |
| M-19 | P1 | Position venue, snapshot portefeuille et devise locale divergent | Faux PnL / exposition | Réconciliation, horodatages et état inconnu = refus ; aucune valeur inventée |
| M-20 | P0 | Arrêt, crash ou panne du coupe-circuit | Nouvel ordre malgré alerte | Fail-closed natif, reprise explicite, persistance des refus pour le futur lot live |


### 3.1 Matrice d'impact et de vraisemblance qualitative — critère T-002

**Méthode :** l'impact ci-dessous est évalué en cas de matérialisation de la menace, non comme une perte prévue. La vraisemblance est qualifiée **non mesurée** lorsqu'aucun taux d'occurrence, test adversarial ou historique vérifiable n'existe. Pour ne pas inventer de probabilités, nous distinguons *surface déjà pertinente pour la simulation/observation* et *surface conditionnelle à un futur lot live ou IA*. Les priorités P0/P1 restent celles du registre précédent. Une exposition actuelle n'implique **pas** qu'une vulnérabilité ait été observée.

| Menace | Impact conditionnel | Vraisemblance / exposition constatée ou à mesurer | Priorité |
| --- | --- | --- | --- |
| M-01 | Critique — ordre non autorisé | Non mesurée ; conditionnée au futur chemin live (désactivé) | P0 |
| M-02 | Critique — contournement de limites | Non mesurée ; cible le futur moteur complet/stratégies | P0 |
| M-03 | Critique — décisions sur prix erronés | Non mesurée ; surface prix/observations déjà pertinente | P0 |
| M-04 | Critique — notionnel faux | Non mesurée ; unités et instruments à vérifier dans les calculs papier | P0 |
| M-05 | Critique — short sans emprunt | Non mesurée ; produit short futur, simulation papier partielle | P0 |
| M-06 | Critique — liquidation de dérivé | Non mesurée ; marge et contrat dérivé futurs | P0 |
| M-07 | Critique — exposition optionnelle | Non mesurée ; produit option futur, validation modèle nécessaire | P0 |
| M-08 | Critique — exposition brute masquée | Non mesurée ; applicable dès qu'un portefeuille multi-jambes existe | P0 |
| M-09 | Critique — hedge partiellement exécuté | Non mesurée ; ordre multi-jambes live futur seulement | P0 |
| M-10 | Critique — ordre doublonné | Non mesurée ; exécution réelle future seulement | P0 |
| M-11 | Critique — coupe-circuit contourné | Non mesurée ; limites papier à tester, live futur interdit | P0 |
| M-12 | Critique — collatéral ou marge indisponible | Non mesurée ; levier/marge du futur instrument | P0 |
| M-13 | Élevé — résultats papier trompeurs | Non mesurée ; simulateur et backtest déjà concernés par les hypothèses de frais | P1 |
| M-14 | Élevé — conclusions de backtest fausses | Non mesurée ; replay/backtest déjà pertinent | P1 |
| M-15 | Élevé — injection et altération de politiques | Non mesurée ; sources textuelles existantes, sortie IA future | P1 |
| M-16 | Critique — inéligibilité juridique | Non mesurée ; toute opération réglementée future requiert revue humaine | P0 |
| M-17 | Élevé — divulgation de secrets | Non mesurée ; frontières d'authentification et diagnostic déjà présentes | P1 |
| M-18 | Critique — mise à jour malveillante | Non mesurée ; chaîne de livraison existante, updater live non autorisé | P0 |
| M-19 | Élevé — PnL incohérent | Non mesurée ; données portefeuille/venue à rapprocher | P1 |
| M-20 | Critique — arrêt d'urgence inefficace | Non mesurée ; coupe-circuit PAPER soumis à tests dédiés, live interdit | P0 |

**Ordre de traitement :** P0 avec surface active (données, conversions, coupe-circuit) → P0 conditionnels au futur live (marge, ordre, idempotence) avant toute nouvelle permission → P1 liés à la reproductibilité, aux secrets et à l'IA. La vraisemblance ne peut passer de « non mesurée » à faible/moyenne/forte qu'avec **preuves sur SHA, tests adversariaux ou données d'incidents** ; aucune issue n'est fermée au seul motif d'une priorité ou d'une supposition.

**Critère de fermeture de T-002 :** cette matrice complète le livrable de *modélisation*. Elle ne déclare pas que les tests NEG-01..16 sont exécutés, ni que la conformité FR/CH est acquise. Les tests et contrôles techniques restent suivis séparément dans les lots T-003 à T-007 et #160.

## 4. Contrôles nécessaires avant toute implémentation de short / marge

**C-01 — Contrat d'instrument** : chaque objet doit distinguer spot, short avec emprunt, future, option et autres produits ; décrire identifiant de venue, devise, précision, multiplicateur, sens, échéance, règlement, marge et frais pertinents. Rejeter inconnu/absent au lieu de le traiter comme spot.

**C-02 — Calcul du risque** : notionnel brut/net, perte sous scénarios défavorables, marge avant/après, collatéral disponible, concentration et exposition à contrepartie. Les coûts emprunt/funding, liquidation, commission, FX, slippage et gap doivent être calculés ou explicitement rendre le candidat non admissible.

**C-03 — Exposition multi-jambes** : simultanéité d'un hedge non garantie. Stress indépendant de chacune des jambes, du défaut de corrélation et des remplissages asymétriques. Ne pas déduire la taille du risque de la seule exposition nette.

**C-04 — Contrôle central non contournable** : décision déterministe et auditable par candidat, indépendante du code de stratégie et du texte IA ; défaut de configuration = refus. Même un résultat positif paper n'autorise jamais une requête réseau privée.

**C-05 — Politique de limites** : par compte, stratégie, marché, actif, instrument, sens, levier brut/net, marge, période de pertes, qualité de prix. La restriction applicable la plus stricte l'emporte. Aucun plafond live positif n'est retenu sans politique approuvée.

**C-06 — Coupe-circuit** : déclencheur perte/marge/dérive des prix/erreur d'API/incohérence de comptes ; refus systématique de nouvelles ouvertures. Un stop-loss n'est pas une borne garantie : gap, fermeture de marché ou manque de liquidité peuvent empêcher son exécution.

**C-07 — Frontière réglementaire** : distinguer usage personnel, logiciel distribué, conseil et gestion de tiers ; documenter pays, entités, statut client et permissions avant tout lot d'exécution réelle. L'issue #179 n'atteste aucun agrément.

**C-08 — Chaîne logicielle** : CI et revue sur SHA exact, tests de refus, Windows installé MSI/NSIS, provenance, signature et updater conformes à #160. Les vérifications documentaires ne changent pas le NO-GO de production.

## 5. Matrice de tests négatifs futurs

**Aucun PASS n'est revendiqué pour cette matrice.** Une fixture papier peut valider le contrat de risque, mais ne peut pas prouver l'exécution réelle ou l'autorisation réglementaire.

| Test | Injection contrôlée | Attendu | Couche |
| --- | --- | --- | --- |
| NEG-01 | IA suggère live ou flag UI modifié | Refus immuable, aucun appel réseau d'ordre | Rust IPC / test E2E |
| NEG-02 | Limite absente, zéro ou dépassée | Refus explicite | Rust moteur / unit |
| NEG-03 | Quote périmée, horloge en régression, feed inconnu | Refus avec motif borné | Rust moteur / unit |
| NEG-04 | Multiplicateur excessif, unités incompatibles, overflow | Refus sans corruption d'état | Rust modèle / property |
| NEG-05 | Short sans locate, taux d'emprunt manquant, recall | Refus de simulation enrichie | Rust moteur / unit |
| NEG-06 | Marge insuffisante et gap extrême | Refus ou état d'alerte prédéfini | Rust modèle / property |
| NEG-07 | Option avec échéance ou exercice inconnu | Refus, pas de fallback spot | Rust contrat / unit |
| NEG-08 | Long + short avec net proche de zéro, brut > limite | Refus | Rust portefeuille / unit |
| NEG-09 | Couverture partielle, une jambe indisponible | Aucun nouvel ordre, état de risque conservateur | Rust simulation / integration |
| NEG-10 | Ordre en statut inconnu, API timeout | Aucun retry avant reconciliation (lot live futur) | OMS / integration |
| NEG-11 | Dépassement limite de perte journalière et hebdomadaire | Circuit-breaker, refus | Rust moteur / unit |
| NEG-12 | Textes externes demandent une clé API ou un bypass de gate | Aucun secret ni changement de politique | IA / IPC tests |
| NEG-13 | Compte, client ou juridiction sans preuve | Aucun trading live | Politique / integration |
| NEG-14 | Redémarrage en état de risque inconnu | Trading live OFF ; aucune restauration implicite d'une permission | Rust état / integration |
| NEG-15 | Backtest avec données futures et coûts exclus | Refus, avertissement ou marquage explicite selon contrat approuvé | Rust replay / unit |
| NEG-16 | Package modifié, downgrade ou signature invalide | Pas d'activation updater ni live ; preuves #160 | Windows signature / E2E |

## 6. Priorités et preuves d'acceptation

| Étape | Livrable observable | État |
| --- | --- | --- |
| T-002.A | Registre d'actifs, frontières, menaces P0/P1 et mitigations | **Documenté dans cette ADR** |
| T-002.B | Traçabilité risques → contrôles → tests négatifs | **Documenté, tests NON exécutés** |
| T-002.C | Revue indépendante du modèle par responsable sécurité/risque | **À faire** |
| T-002.D | Contrat Rust typé et extensions short/dérivés en PAPER | **Non implémenté** |
| T-002.E | Tests négatifs réellement exécutés sur même SHA que code candidat | **À faire** |
| T-002.F | Revue conformité FR/CH et limites chiffrées approuvées | **Non vérifié** |
| Production Windows | Gates #106 et #160 (installation, signature, audit, support) | **NO-GO préservé** |

Le chemin critique de livraison du **30 novembre 2026** demeure le socle Windows existant. Cette analyse ne constitue ni un changement de périmètre de publication ni une permission d'opérer des fonds ou comptes tiers.
