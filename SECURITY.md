# Sécurité · Безопасность · Security

## Français

Cette politique décrit le serveur Rust maintenu (`rust/pravda-server`). Les protections de l'archive Python ne sont pas automatiquement celles du serveur Rust.

### Contrôles présents

- Écoute sur `127.0.0.1` uniquement, port validé et réservé avant le démarrage des flux.
- Vérification de `Host` : adresse loopback ou `localhost`, avec le port configuré. Une origine fournie doit correspondre exactement à l'hôte et au port de la requête HTTP locale. Les origines externes, `null`, les en-têtes de sécurité dupliqués et les requêtes navigateur inter-origines sont refusés avant les routes HTTP et WebSocket.
- Les clients locaux sans en-tête `Origin` restent autorisés. Les en-têtes de proxy ne créent aucune autorisation.
- `Cache-Control: no-store`, `nosniff`, interdiction d'intégration en frame, politique de référent restrictive et isolation des ressources par origine sur les réponses du serveur, y compris les refus.
- Validation des prix finis et strictement positifs ; contrôle de fraîcheur monotone, désynchronisation et plafond d'anomalie avant simulation.
- Requêtes SQLite paramétrées, historique filtré et borné. Export CSV de champs contrôlés : symboles/exchanges énumérés, prix positifs et timestamps validés.
- Tests Rust locaux et workflow CI Windows/Linux, avec contrôle de compatibilité Rust 1.95.0. Un résultat CI doit être rattaché au SHA effectivement contrôlé.

### Limites connues

- Le dashboard n'a **pas d'authentification**. Ces contrôles ne protègent pas contre un programme malveillant déjà exécuté sur le poste, qui peut forger Host/Origin. Ne pas exposer le serveur par un proxy, un tunnel public ou une redirection de port.
- La CSP autorise encore les scripts/styles inline. La migration vers des ressources externes ou des hashes reste à faire. Les contrôles d'origine ne remplacent pas la prévention des injections dans l'interface.
- Une limitation globale des requêtes, des connexions WebSocket et de l'utilisation disque reste à implémenter. Le bornage d'une réponse ne constitue pas une protection complète contre le déni de service.
- Les transports HTTP/WebSocket des sources, le lanceur Tauri et les archives nécessitent leurs propres contrôles. Aucun audit général certifiant l'absence de vulnérabilités n'est revendiqué.
- Le modèle papier ne vérifie ni comptes, ni inventaires, ni exécution réelle. Le ticker Kraken v1 est horodaté à la réception, sans preuve de l'âge de la transaction à la source.
- Le serveur lit les variables de processus ; il ne charge pas automatiquement `.env`. Les connecteurs ntfy/LLM et plusieurs sources restent en cours de migration.

### Secrets et signalement

Ne pas commiter de clés, topics privés, bases de données ou journaux personnels. `.gitignore` n'efface pas un secret déjà présent dans l'historique : le révoquer s'il a été exposé. Aucun filtre ne garantit qu'une sortie arbitraire est exempte de secrets.

Pour signaler une vulnérabilité, utiliser le signalement privé GitHub si cette fonction est activée dans l'onglet Security du dépôt. Sinon, demander au mainteneur un canal privé sans publier la preuve d'exploitation ni les données sensibles. Aucun délai de réponse ou canal privé non vérifié n'est promis.

## Русский

Политика относится к Rust-серверу, а не к архиву Python. Сервер слушает только loopback, проверяет Host/Origin и браузерный контекст запроса до HTTP/WebSocket-маршрутов. Локальные CLI-клиенты без Origin разрешены. Ответы запрещают кэширование и содержат защитные заголовки.

Аутентификации нет: локальная вредоносная программа может подделать заголовки. Не публикуйте сервер через туннель или прокси. CSP пока допускает inline-скрипты; ограничение нагрузки и диска ещё не завершено. Наличие тестов не означает отсутствие уязвимостей. Сообщайте о проблемах приватно, не публикуя секреты.

## English

This policy covers the maintained Rust server, not inherited Python protections. The server binds to loopback and validates Host/Origin plus browser request context before HTTP/WebSocket routes. Local CLI requests without Origin remain allowed. Responses use no-store and defensive headers.

There is no authentication: local malware can forge headers. Do not expose the server through a tunnel or proxy. CSP still allows inline scripts, and global request/connection/storage limits remain unfinished. Tests do not establish the absence of vulnerabilities. Use GitHub private reporting if enabled, or request a private maintainer channel without disclosing secrets publicly.
