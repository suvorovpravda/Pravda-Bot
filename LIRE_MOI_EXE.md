# Pravda-Bot : lanceur Windows .exe

Ce kit construit `Pravda_Lanceur.exe`, un lanceur pour le projet et son
environnement `.venv` déjà installés sur votre PC. La construction s'effectue
une fois sous Windows avec Python 3.12. Les lancements suivants se font par
double-clic sur l'exécutable, sans VS Code ni commandes PowerShell.

## Installation

1. Extraire **tout le contenu** du ZIP dans
   `C:\Users\Utilisateur\Pravda-Bot`, à côté de `main.py` et `dashboard.py`.
   Il faut y retrouver `pravda_launcher.py`, `Construire_Pravda_EXE.cmd`
   et le dossier `launcher_tests`.
2. Double-cliquer sur `Construire_Pravda_EXE.cmd`. Ce script utilise
   `.venv\Scripts\python.exe`, installe PyInstaller dans ce venv, teste le
   lanceur puis crée `Pravda_Lanceur.exe` à la racine du projet.
3. Démarrer LM Studio avec son serveur local sur `127.0.0.1:1234`, puis
   double-cliquer sur `Pravda_Lanceur.exe`.

Les dépendances de `requirements.txt` doivent déjà être installées, comme
dans l'installation réalisée précédemment. Le modèle chargé dans LM Studio
doit correspondre à `LM_MODEL_NAME` dans l'environnement ou le `.env` existant.
À défaut de valeur, le code du bot utilise `qwen/qwen3.5-9b`.

Le projet et `.venv` restent nécessaires à côté de cet exécutable. Pour un accès
depuis le Bureau, créer un raccourci vers l'exécutable situé dans Pravda-Bot.
Le kit n'embarque pas les sources du bot, son `.env`, ses CSV ou ses secrets.

## Déroulement du lancement

Le lanceur ouvre une console, valide la configuration et les dépendances,
puis exécute la suite de tests actuellement présente dans `tests`. Un échec
interrompt le démarrage. Le lanceur ne modifie pas les tests du bot.

Après réussite, il démarre un processus de dashboard qui écoute sur
`127.0.0.1` avec un port attribué par Windows. Le véritable socket du serveur
reste ouvert ; il n'y a pas de sondage suivi d'une fermeture puis d'un nouveau
bind. Le nom d'hôte et le port du navigateur sont construits localement à
partir du numéro de port confirmé par ce processus.

Le lanceur démarre ensuite le bot et ouvre le dashboard dans le navigateur.
La console affiche les deux adresses exactes, par exemple :

```text
Dashboard : http://127.0.0.1:52341
Metriques : http://127.0.0.1:52341/api/metrics
```

Le numéro est un exemple. Il peut changer à chaque lancement. Aucun réglage
de `DASHBOARD_PORT` n'est requis pour ce nouveau lanceur. Les applications
qui occupent déjà 8080 ou 8081 sont conservées.

Fermer d'abord les anciennes sessions de Pravda-Bot pour éviter plusieurs bots
écrivant dans les mêmes journaux. Une protection empêche l'ouverture simultanée
de deux instances de **ce lanceur** pour le même dossier.

## Simulation et données

Le mode de simulation est `portfolio` : solde initial de 1 000 USDT virtuels,
allocation de 10 %, seuil brut de 0,05 %, plafond de 5 %, confirmation de 3 s,
hystérésis de 0,05 point, fraîcheur maximale de 4 s et désynchronisation maximale
de 3 s. Le modèle de frais du bot reste inchangé à 0,36 point de pourcentage.
Le code du bot ne passe pas d'ordres réels. `TRADING_MODE=paper` est son
étiquette d'affichage, pas un mécanisme de désactivation d'ordres.

`NTFY_TOPIC_URL` est forcée à une chaîne vide dans les processus créés,
avant le chargement de `.env`. Les notifications ntfy sont donc désactivées
pour cette première version du lanceur. Le `.env` n'est pas modifié.
L'adresse LM Studio est fixée à `http://127.0.0.1:1234/v1`.

Les CSV sont lus et écrits dans le dossier de Pravda-Bot. Le redémarrage du
bot réinitialise son portefeuille papier et ses positions en mémoire.

## Arrêt et erreurs

Utiliser `Ctrl+C` dans la console du lanceur : il demande l'arrêt de ses deux
processus et attend jusqu'à 15 secondes. Si un processus ne répond pas, il
termine uniquement ce processus qu'il a créé. Aucun processus trouvé sur un
autre port n'est tué. Sous Windows, un Job Object regroupe aussi les processus
créés pour les fermer si le lanceur disparaît.

Si le bot ou le dashboard s'arrête de façon inattendue, le lanceur ferme son
autre processus et affiche l'erreur. La console reste ouverte pour lecture.

Une route `/api/metrics` en HTTP 503 reste un échec de collecte psutil à
diagnostiquer. Le lancement du serveur HTTP ne prouve pas le succès de cette
collecte. Ouvrir l'adresse de métriques affichée et vérifier HTTP 200,
`status: success`, un uptime qui augmente et une mémoire résidente cohérente.
Le CPU peut rester à zéro au repos. Ces mesures concernent le dashboard.

## Sources, tests et construction

Le lanceur utilise uniquement la bibliothèque standard. Il n'importe pas le
bot dans le processus figé : il utilise le Python de `.venv` avec les sources
du projet. Le chemin de recherche des DLL de PyInstaller est réinitialisé
pendant la création des processus externes, puis restauré dans le lanceur.
Les commandes sont transmises à `subprocess.Popen` sous forme de listes,
sans interpréteur de commandes. Aucune donnée `.env` n'est collectée par
les options de construction.

Les fichiers générés pendant la construction sont `.pravda-exe-build`,
`.pravda-exe-spec` et `Pravda_Lanceur.exe`. Ils peuvent être ignorés dans Git ;
les sources à conserver sont les trois fichiers du kit et `launcher_tests`.

Pour relancer uniquement les tests du lanceur sous PowerShell :

```powershell
.\.venv\Scripts\python.exe -m unittest discover -s launcher_tests -p 'test_*.py' -v
```

Validation effectuée sous Linux avec Python 3.12 : 14 tests découverts,
12 réussis et 2 contrôles spécifiques à Windows ignorés. Les scénarios
comprennent un démarrage avec un port demandé déjà occupé, la réservation
du vrai port jusqu'à l'arrêt, l'arrêt gracieux, le refus de démarrer lorsque
les tests échouent, l'arrêt du dashboard si le bot quitte et le maintien de
ntfy désactivé malgré un topic factice présent dans `.env`.

Le contrôle du dossier de travail compare l'identité du dossier avec
`Path.samefile()`. Il accepte les noms longs et leurs alias Windows 8.3,
par exemple `Utilisateur` et `UTILIS~1`, lorsque ces chemins désignent le même
dossier, tout en rejetant un véritable changement de répertoire.

Un essai local supplémentaire du worker avec le véritable gestionnaire
`DashboardHandler` de l'archive auditée a renvoyé HTTP 200 pour `/`, `/data`
et `/api/metrics`. Il valide l'intégration HTTP dans cet environnement,
sans mesurer les performances ni valider l'exécutable Windows.

Les deux contrôles Windows sont rejoués par le script de construction :
Job Object limité aux processus assignés et réinitialisation du chemin DLL
pour un sous-processus d'une application figée. La construction de l'exécutable
et son fonctionnement sous Windows restent à vérifier sur votre PC.

Le kit est livré comme source et script de construction, avec des tests.
Il ne contient pas de binaire Windows compilé dans cet environnement Linux.
PyInstaller nécessite une construction sur le système cible.

Références officielles :

- [PyInstaller : systèmes de construction](https://www.pyinstaller.org/en/stable/)
- [PyInstaller : option onefile et chemins de sortie](https://www.pyinstaller.org/en/stable/usage.html)
- [PyInstaller : lancement de programmes externes sous Windows](https://pyinstaller.org/en/latest/common-issues-and-pitfalls.html)
- [Microsoft : Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
