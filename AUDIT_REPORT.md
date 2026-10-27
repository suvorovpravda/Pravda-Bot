# Rapport d’Audit Cybersécurité – Pravda-Bot
**Date** : 2026-10-27  
**Périmètre** : Code source, configuration, dépendances, GitHub `suvorovpravda/Pravda-Bot`

## Résumé exécutif
Audit global de sécurité, hardening et refonte terminal. Application locale de surveillance publique, sans ordre réel. Risques critiques liés aux dépendances, recommandations de chiffrement au repos et surveillance continue.

## Méthodologie
* Revue de code complète, OWASP Top 10
* Analyse dépendances, pip-audit
* Tests unitaires : 116 tests OK
* Validation configuration, secrets, logs
* Cross-check intégrité flux

## Résultats par criticité

### Critique
**R-01 Dépendance pip 25.0.1 vulnérable dans ancien venv**
* Impact : vulnérabilités connues dans l’environnement d’exécution
* Correctif : bootstrap pip 26.2.2 appliqué, requirements avec hashes, venv propre requis
* Statut : Mitigé – migration venv propre nécessaire

### Moyen
**R-02 Chiffrement au repos absent pour SQLite/CSV**
* Impact : données locales en clair sur disque
* Recommandation : chiffrer market_history.sqlite3 si usage persistant sensible

**R-03 Modules sécurité proposés non revus**
* Impact : SSRF, fuite mémoire, faux positifs
* Correctif : non intégrés, `SUGGESTIONS_SECURITE_A_EXAMINER.txt` conservé pour revue future

**R-04 Dépendances tierces sans surveillance continue**
* Impact : dérive de vulnérabilités
* Recommandation : CI avec pip-audit, bandit, renouvellement trimestriel hashes

### Faible
**R-05 Dashboard sans auth**
* Mitigé par bind 127.0.0.1, validation Host/Origin
**R-06 Logging**
* Aucun secret loggé, NTFY_TOPIC_URL forcé vide au démarrage

## Améliorations appliquées
* README pro/modern publié
* Terminal enrichi : historique multi-échelle, FX source/âge, état sources, alertes EUR/CHF, export quotidien
* SECURITY.md ajouté avec politique hardening
* Tests 116 OK, pip check OK

## Recommandations long terme
* CI complète : pip install --require-hashes, pip check, pip-audit, compileall, tests
* Monitoring fraîcheur sources avec seuils d’alerte
* Rotation sujet ntfy, gestion secrets .env
* Revue trimestrielle dépendances

## Validation
* Tests unitaires : Ran 116 tests in 8.640s OK
* Pas de régression fonctionnelle
* Zero Footprint respecté

**Conclusion** : Architecture conforme pour usage local de surveillance publique. Risques résiduels maîtrisés avec les recommandations ci-dessus.
