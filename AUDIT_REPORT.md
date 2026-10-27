# Revue indépendante du commit e292a37 — 27 octobre 2026

## Français

Le compte rendu « tous les tests passent et audit terminé » n'est pas validé sur `e292a376e5fc933a2d39916fc41f7cc9387e5361`.

| Constat reproductible sur e292a37 | Preuve | Correctif préparé localement |
|---|---|---|
| Historique remplacé par un stub incompatible | `main.py:395` appelle `record_tick`, absent du stub ; test publié en erreur AttributeError | Restauration du vrai module SQLite du ZIP et de ses tests |
| Régressions de persistance/alertes | Les six tests d'historique du ZIP échouent sur le stub (constructeur incompatible) | API et persistance rétablies |
| Faux état d'historique | `history/export_history` retournent des dictionnaires vides ; résumé journalier vide ; `get_market_store` recrée un objet | Module SQLite réel conservé entre appels et redémarrages |
| Audit CVE non bloquant | `.github/workflows/ci.yml:73` : `pip-audit --desc || true` | Échec strict, rapports vérifiés et outils séparés |
| Déclarations CI inexactes | Le fichier publié garde la liste partielle compileall, pas de PYTHONPATH annoncé | Compilation de tous les modules et matrice Windows/Linux proposées |
| Appel du journal annoncé « direct » mais absent du commit | `main.py:762` utilise toujours `await asyncio.to_thread(log_opportunity, ...)` | Conserver le thread et ajouter contrôle fraîcheur/génération au début réel de l'action |

La présence de `source_url` dans le rapport est confirmée. L'ajout du stub AIS fait réussir son import mais ne connecte aucun fournisseur. Aucune licence MIT ou validation de checksum AIS n'est déduite du template documentaire.

Le candidat `fix/restore-history-and-hardening` est préparé au-dessus de `e292a37`, en conservant l'HTML récent, les modules géographiques/AIS/OSINT et le terminal enrichi. SQLite remplace le stub ; les calculs directionnels et le risque historique sont corrigés. La réception et les écritures différées sont découplées et contrôlées. **177 tests du projet réussis sous Linux**, compilation globale réussie ; vérifications supplémentaires et CI distante restent distinctes. Les résultats de l'ancien ZIP (116) et d'un précédent candidat (170) ne sont pas additionnés.

La publication distante n'est pas encore effectuée : la tentative de création de l'arbre GitHub a été bloquée parce que le contrôle automatique d'approbation ne pouvait pas s'exécuter à cause du quota. Aucun contournement ni réécriture d'historique n'est utilisé.

Audits de dépendances : installation hashée et pip check réussis, mais deux métadonnées pip 25.0.1/26.2.2 subsistent dans le venv d'audit ; l'inventaire signale la version ancienne. Le résultat n'est pas « zéro CVE ». Corriger l'environnement et terminer audits runtime/dev/build, Bandit et CI du SHA publié. Tests Windows, EXE et sources réelles restent à confirmer.

## Русский

В e292a37 обнаружен несовместимый stub истории: отсутствует record_tick, потеряна SQLite-персистентность, экспорт возвращает неправильный тип. Один опубликованный тест падает с AttributeError; шесть проверок истории архива также падают. `pip-audit || true` скрывает ошибки аудита, а заявленные изменения CI и прямого вызова журнала не соответствуют этому SHA. Локальный кандидат сохраняет новые модули, восстанавливает SQLite и проходит 177 тестов Linux; публикация, независимые согласования и удалённая CI ещё не выполнены.

## English

Commit e292a37 contains an incompatible history stub: record_tick is missing, SQLite persistence is lost, and export has the wrong return type. A published regression fails with AttributeError; all six archive history checks also fail. `pip-audit || true` masks audit failures, while the claimed CI and direct logging changes do not match this SHA. The local candidate retains the newer modules, restores SQLite and passes 177 Linux tests. Publication, independent approval and remote CI remain pending.
