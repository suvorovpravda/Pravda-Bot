<div align="center">

# 🔒 Sécurité · Безопасность · Security

*Périmètre documenté · Contrôles vérifiables · Limites explicites*

[![Python](https://img.shields.io/badge/Python-3.12-3776AB?style=flat-square)](https://www.python.org/)
[![CI](https://github.com/suvorovpravda/Pravda-Bot/actions/workflows/ci.yml/badge.svg?branch=fix%2Frestore-history-and-hardening)](https://github.com/suvorovpravda/Pravda-Bot/pull/2/checks)

</div>

## Français

### Périmètre et modèle de menace

Pravda-Bot est une application locale de surveillance publique et de simulation papier. Elle ne passe pas d'ordres réels et ne valide pas les soldes ou les permissions d'un compte privé. Le dashboard utilise `http.server`, écoute sur **127.0.0.1** et n'a pas d'authentification. Il ne doit pas être publié par tunnel, interface réseau publique ou reverse proxy public.

La surface examinée inclut les données externes, les requêtes d'un navigateur vers le serveur local, les écritures différées, les dépendances et les fichiers d'exécution. Un utilisateur ou programme pouvant modifier les sources ou le `.env` possède déjà un accès important au poste ; cette application n'est pas une sandbox contre cet accès.

### Contrôles des chemins maintenus

| Surface | Contrôle appliqué | Limite |
|---|---|---|
| HTTP local | Host/port réel, Origin des mutations, rejet des requêtes cross-site, rate limiter borné, délais et tailles maximales | Sans authentification ; un programme local reste dans le périmètre de confiance du poste |
| Navigateur | CSP avec hashes des scripts inline, titres externes rendus en texte, protocoles des liens vérifiés | Styles inline conservés ; toute nouvelle route ou ressource doit être revue |
| HTTP sortant | Hôtes exacts, HTTPS public, absence de redirections, corps lu sous plafond, retries GET bornés | Vérification DNS préalable sans pinning IP ; pas d'annulation absolue de DNS/en-têtes lents |
| LM Studio | Exception HTTP explicite pour loopback et port configuré ; désactivé dans l'exemple | Ni serveur distant arbitraire ni fallback cloud automatique |
| RSS | defusedxml ; DTD, entités et références externes interdites | Une source autorisée peut rester erronée ou indisponible |
| Cotations | Prix finis/positifs, générations et fraîcheur/synchronisation revalidées avant effets différés | Les derniers prix ne prouvent pas l'exécution d'un ordre |
| CSV | Texte de modèle borné, cellules texte protégées contre les formules, lecture récente bornée et consciente des guillemets | Une observation individuelle trop grande ou invalide est refusée ; le stockage disque peut croître |
| Dépendances | Bootstrap/runtime/audit/build séparés, versions et hashes, pip check, audits stricts en CI | CVE connues seulement ; les hashes ne prouvent pas qu'un paquet est bienveillant |
| Processus | Arguments en liste et gestion d'arrêt/propriété des enfants dans le lanceur | Le build `--help` n'est pas une validation de tous les scénarios sur le PC utilisateur |

Les POST ntfy/LLM ne sont pas rejoués par le client HTTP commun. CCXT, WebSocket et les utilitaires expérimentaux possèdent leurs propres transports et nécessitent leur propre revue. Le client historique `src/lm_studio_client.py` n'est pas le chemin LLM actif durci ; il reste explicitement hors d'une affirmation de couverture totale.

### Secrets et fichiers locaux

- `.env`, venvs, bases SQLite/WAL, CSV privés, journaux et EXE restent hors de Git. Un fichier déjà suivi n'est pas retiré par la seule présence d'une règle `.gitignore`.
- `.env.example` contient des valeurs d'exemple et `NTFY_TOPIC_URL=` vide. Le lanceur de démonstration désactive les notifications ; un lancement direct peut utiliser une destination configurée par l'utilisateur.
- Les erreurs des chemins HTTP maintenus sont génériques. Il ne faut pas afficher les exceptions réseau brutes, les URL privées, les headers ou les corps de réponse dans les journaux.
- Aucun filtre de logs ne garantit la suppression de tous les secrets dans une sortie arbitraire. Examiner les fichiers destinés à Git, y compris les fichiers non suivis, avant de les ajouter.
- Si une clé ou un topic privé est exposé, le retirer du code ne suffit pas : révoquer ou renouveler l'accès auprès du fournisseur et examiner les copies déjà diffusées.

SQLite n'est pas chiffré par l'application. Permissions du poste, sauvegardes et éventuel chiffrement du volume doivent être gérés selon l'usage. La lecture bornée des observations ne constitue pas une politique de rétention des bases ou journaux ; les exports historiques volumineux demandent encore une surveillance spécifique.

### Vérification et publication

La CI vérifie les versions réellement installées, la syntaxe des modules publiés, JavaScript, les tests Python/lanceur, les audits CVE et Bandit. Un outil absent, un téléchargement interrompu, un paquet ignoré ou un audit inconcluant doit rester un échec explicite. Aucun `|| true` ne doit masquer pip-audit.

Les résultats sont attachés au SHA concerné. Une CI verte ne prouve pas l'absence de vulnérabilité, la disponibilité de Kraken/Binance/AIS, la qualité d'un modèle, une conformité OWASP globale ou des droits de redistribution. Une différence de timing doit être rendue reproductible et corrigée ; un passage en CI n'est pas un motif pour ignorer un échec local connu.

La fusion et les releases suivent le circuit de revue défini dans le README. Aucune approbation Gemini, GPT ou humaine n'est inventée. Pas de force push ni d'identité Git fabriquée pour débloquer une livraison.

### Géographie, AIS et actualités

Les modules expérimentaux ne prouvent pas une collecte AIS opérationnelle. Le proxy vide est indiqué indisponible, sans positions fabriquées. Couverture, dates, classification des navires et conditions du fournisseur doivent être vérifiées avant toute utilisation ou redistribution. Aucun caractère gratuit, non commercial ou purement terrestre d'un fournisseur n'est présumé à partir du seul nom d'un module.

Les points ENTSOG et les segments proxy ne prouvent pas un tracé de conduite. Les articles restent des informations rapportées ; la présence d'un lien officiel ne confirme pas à elle seule un incident. Aucun cours TTF licencié ou Forex interbancaire continu n'est annoncé.

### Signaler une vulnérabilité

Utiliser le signalement privé GitHub **s'il est activé**. Sinon, demander au mainteneur un canal privé avant d'envoyer les détails sensibles. Aucun email ou canal interne non vérifié n'est annoncé.

Fournir le SHA, le composant, une reproduction minimale et l'impact. Masquer clés, topics, données personnelles et identifiants de compte. Ne pas joindre `.env`, base privée ou journal complet à une issue publique.

## Русский

### Область применения

Приложение локальное: публичные источники и бумажная симуляция, без реальных ордеров или проверки балансов. Dashboard без аутентификации должен оставаться на **127.0.0.1**. Не открывайте его через публичный интерфейс, туннель или reverse proxy.

Поддерживаемые пути проверяют Host/Origin, размеры, время, точные HTTP-хосты и свежесть котировок. Используются CSP, защищённый RSS-парсер и ограниченное чтение последних логических CSV-записей. Это уменьшает риск, но не является полной защитой. DNS-проверка не закрепляет IP соединения; сторонние транспорты и экспериментальные модули требуют отдельной проверки.

### Секреты, зависимости и данные

`.env`, базы, журналы, venv и EXE не публикуются. Ошибки HTTP должны быть общими, без частных URL или тела ответа; фильтр журналов не гарантирует удаление произвольного секрета. При утечке необходимо отозвать или заменить доступ у поставщика. SQLite не шифруется самим приложением, а объём хранения требует отдельной политики.

Зависимости и инструменты разделены, закреплены с хешами и проверяются в CI. Неудачный или незавершённый аудит нельзя превращать в успех. Зелёная CI относится к конкретному SHA и не доказывает отсутствие всех уязвимостей, наличие лицензии, реального AIS-потока или корректность геополитического вывода. Нестабильный тест необходимо исправлять, а не отключать из-за успешного CI-прогона.

### Приватное сообщение

Используйте приватный GitHub-канал, если он включён, либо сначала запросите подтверждённый приватный контакт сопровождающего. Укажите SHA, минимальный сценарий и последствия. Не прикладывайте секреты, личные данные, `.env`, БД или полные журналы к публичной issue.

## English

### Scope

Local public-data monitoring and paper simulation only: no real orders or balance validation. The unauthenticated dashboard must remain on **127.0.0.1**. Do not expose it through a public interface, tunnel or reverse proxy.

Maintained paths apply Host/Origin checks, request/time/size bounds, exact HTTP targets and quote-freshness checks. CSP, hardened RSS parsing and bounded reads of recent logical CSV records reduce risk without establishing complete security. DNS prechecks do not pin the connection IP. Third-party transports and experimental utilities require separate review.

### Secrets, dependencies and data

Keep `.env`, databases, private logs, venvs and executables out of Git. Maintained HTTP paths return generic errors; arbitrary-output redaction cannot guarantee complete secrecy. Exposed keys/topics require provider-side revocation or rotation. SQLite is not encrypted by the application, and storage growth needs its own retention plan.

Runtime, bootstrap, audit and build dependencies are separately pinned and hashed. Failed or inconclusive audits must not be hidden. A green CI result belongs to one SHA; it does not prove complete security, provider availability, data rights or incident verification. Known flaky assertions should be made deterministic rather than skipped because CI happens to pass.

### Reporting

Use GitHub private vulnerability reporting if enabled; otherwise obtain a verified private contact from the maintainer before sharing sensitive details. Include the SHA, affected component, minimal reproduction and impact. Exclude credentials, personal data, `.env`, databases and full logs from public issues.
