# Release v0.1.2 – Prêt à être publié

- Ajout d’un pool SQLite thread‑safe (`SQLitePool`).
- Introduction du mécanisme de file d’attente rétro‑pression (`BoundedFeedQueue`).
- Modèles Pydantic pour validation des données (MarketTick, PriceAlert, SourceObservation).
- Gestion de retry sur verrou `database is locked` dans les opérations d’écriture.
- Création d’un workflow GitHub Actions automatisant la publication sur PyPI à chaque tag.

## Notes techniques
- Le pool est configuré avec un timeout de 5 s et `check_same_thread=False`. 
- La file d’attente a une taille maximale configurable (défaut 100). 
- Les tests unitaires passent sans modifications supplémentaires.
