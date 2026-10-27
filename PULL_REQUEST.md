# PR feat/local-enhancements

## Contexte
Reprise à zéro après handoff. Installation propre validée, branche créée depuis origin/main 45380cd.

## Changements
- Lancer_Pravda_Bot.bat refondu
- dashboard.html/dashboard.py enrichis
- .gitignore + market_history.sqlite3*
- ROADMAP.md ajouté
- README.md réécrit pro/modern

## Validation
- Tests unitaires passent : Ran 106 tests OK
- Sécurité de base vérifiée : pas de 0.0.0.0, shell=True, verify=False, secrets
- Installation propre Python 3.12 validée

## Limites
- Modules sécurité non intégrés sans revue
- Build Windows non testé ici

## Checklist
- [x] Tests passent
- [x] README pro
- [x] ROADMAP.md
- [ ] CI verte
- [ ] Revue Gemini
