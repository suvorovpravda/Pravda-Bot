PRAVDA-BOT - MODE NAVIGATEUR WINDOWS
=================================
1. Extraire l'archive ZIP complete (ne pas lancer depuis l'archive).
2. Double-cliquer sur "Demarrer-Pravda-Bot.cmd".
3. Le navigateur par defaut ouvre http://127.0.0.1:8080/login
4. Definir un mot de passe de 15 caracteres minimum si demande.
5. Pour fermer le serveur, double-cliquer "Arreter-Pravda-Bot.cmd".

ATTENTION : pravda-server.exe doit rester dans le MEME dossier
que les fichiers .cmd et .ps1. Ce mode n'utilise pas la fenetre Tauri
qui a pose probleme. Il ne requiert ni Rust ni Python sur votre PC.

Les donnees existantes ne sont ni reinitialisees ni supprimees :
%LOCALAPPDATA%\Pravda-Bot\pravda.db
%LOCALAPPDATA%\Pravda-Bot\pravda_auth.json
Journaux techniques (en cas de besoin) :
%LOCALAPPDATA%\Pravda-Bot\logs\

Si le port 8080 est utilise, fermer les anciennes instances avant
de relancer. Ne jamais transmettre le fichier d'authentification.
Les journaux et la base de donnees peuvent contenir des donnees privees.

Pravda-Bot est une simulation papier, jamais une application de
passage d'ordres reels.
