# Feuille de route — idées pour les prochaines versions

Les pistes ci-dessous sont des améliorations envisagées, pas des fonctions déjà disponibles. Leur ordre pourra évoluer selon les sources de données accessibles, leurs licences et les besoins des utilisateurs.

## Priorité 1 — Un terminal plus utile au quotidien

- **Historique de prix persistant** : conserver les ticks et produire des graphiques multi-échelles (jour, semaine, mois), avec export CSV.
- **Prix crypto en EUR et CHF plus lisibles** : montrer l’heure et la source du taux de conversion, son âge et la devise de cotation d’origine ; proposer des alertes de seuil configurables dans la devise choisie.
- **État des sources sur une seule vue** : distinguer clairement « live », « retardé », « périmé » et « indisponible », avec heure du dernier événement pour chaque plateforme et fournisseur.
- **Résumé quotidien exportable** : mouvements de marché, événements énergétiques observés et liens vers les sources, sans transformer les articles en conclusions automatisées.

## Priorité 2 — Carte géographique et données énergie

- **Carte géographique réelle des infrastructures gazières** : afficher pipelines et terminaux à partir de données géographiques vérifiables, avec attribution, date, précision et droits de réutilisation. Les points ENTSOG actuels ne suffisent pas à reconstituer le tracé des conduites.
- **Routes commerciales des navires GNL** : visualiser positions et trajets uniquement après intégration d’un fournisseur AIS adapté, avec sa clé, ses conditions d’utilisation et ses droits de redistribution. Les positions ne doivent pas être extrapolées ni inventées.
- **Prix TTF vérifié** : intégrer un fournisseur autorisé de prix de marché, indiquer contrat/échéance, devise, unité, horodatage, statut retardé ou live et conditions de licence. En attendant, le tableau de bord continuera à afficher l’indisponibilité plutôt qu’un prix estimé présenté comme réel.
- **Contexte énergétique** : ajouter, lorsque les sources appropriées le permettent, niveaux de stockage, capacité et utilisation des terminaux GNL, incidents publiés et données météo pertinentes, chaque série avec son unité et son intervalle de publication.

## Priorité 3 — Veille géopolitique prudente

- **Carte mondiale d’événements** avec sources complémentaires (par exemple GDELT, ACLED et fournisseurs d’actualités), liens vers les publications originales, dates, zones géographiques et dédoublonnage traçable.
- **Indicateurs de contexte par pays** clairement expliqués : couverture des sources, période observée, méthode de calcul et incertitude. Un score de couverture médiatique ne doit pas être présenté comme une mesure certaine du risque réel.
- **Filtres et suivi personnalisé** par zone, sujet et période, avec notifications configurables et possibilité de corriger les erreurs de géolocalisation ou de catégorisation.
- **Journal de provenance** pour retrouver quelle source et quelle règle ont produit chaque élément affiché.
