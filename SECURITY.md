# Politique de sécurité · Политика безопасности · Security policy

## Summary (English)
- OWASP Top 10 mitigations implemented:
  - Input validation & sanitization
  - Least privilege for secrets
  - Secure transport (HTTPS only)
  - No authentication on public endpoints
  - Logging and monitoring
- Secret management: .env files excluded, secrets never committed
- Dependency audit: pip-audit used; failures are not ignored

## Français

### Périmètre

Pravda-Bot observe des sources publiques et effectue une simulation papier. Aucun ordre réel ni accès aux soldes d'exchange n'est implémenté. Le dashboard reste local sur **127.0.0.1**, sans authentification ; il ne doit pas être exposé par interface publique, tunnel ou reverse proxy public. Une écoute loopback réduit l'exposition ; elle ne supprime pas les menaces venant d'un navigateur ou d'un programme local.

### Contrôles de ce candidat

- Les routes du dashboard vérifient l'autorité Host et le port réel. Les mutations exigent une origine locale cohérente. Des limites de requêtes, tailles, temps et CSV s'appliquent ; elles ne remplacent pas une authentification.
- CSP des scripts inline par hashes, absence de rendu HTML des titres externes et contrôle des protocoles des liens ; styles inline conservés pour l'interface existante.
- Le client HTTP maintenu limite les hôtes exacts, impose HTTPS public et interdit les redirections. LM Studio utilise une exception HTTP loopback. Réponses bornées pendant la lecture et POST non rejoués. Le contrôle DNS préalable n'est pas un pinning IP ; DNS et en-têtes lents ne sont pas annulables de façon absolue par ce client synchrone.
- Les sources RSS utilisent defusedxml avec DTD, entités et références externes interdites.
- Le texte de modèle et les cellules texte des rapports sont bornés et protégés contre les formules de tableur. Les erreurs des chemins HTTP maintenus sont génériques ; aucun masquage ne garantit la confidentialité d'une sortie arbitraire.
- `.env`, bases SQLite/WAL, CSV privés, venvs, journaux et EXE restent hors de Git. `.env.example` ne contient pas de secret et désactive ntfy par défaut. `AIS_API_KEY` est affichée une seule fois sur aisstream.io/account : la stocker uniquement dans une variable d’environnement ou un secret manager, jamais dans le code ou le repo.
- Bootstrap, runtime, audit et build utilisent des verrous séparés avec hashes. Les mises à jour demandent une revue et une installation propre ; l'audit CVE doit échouer en cas de vulnérabilité ou de résultat inconcluant. Aucun `|| true` ne doit transformer l'échec de pip-audit en succès.
- Fraîcheur et génération des cotations sont vérifiées avant les effets différés. Un résultat papier n'est ni une autorisation de trading ni une preuve de rentabilité.

### Limites restant à vérifier

Les transports propres à CCXT/WebSocket, les utilitaires expérimentaux et les nouvelles sources nécessitent leur propre revue. Le client historique `src/lm_studio_client.py` n'est pas le chemin LLM actif durci. L'audit de l'environnement local a trouvé deux métadonnées pip concurrentes ; il ne permet pas d'annoncer « zéro CVE ». Windows/EXE, fournisseurs réels, licences AIS, couverture et modèle LM Studio ne sont pas validés par des tests Linux seuls. Le stub AIS retournant une liste vide ne démontre aucune collecte.

La base SQLite n'est pas chiffrée par l'application ; ses permissions, sa sauvegarde et sa taille relèvent de l'exploitation locale. Les articles sont des informations rapportées ; les points/correspondances géographiques ne prouvent ni incident ni route réelle. Aucune conformité OWASP globale ou absence totale de vulnérabilité n'est revendiquée.

### Signaler un problème

Ne publier aucun secret, `.env`, base privée ou journal intégral dans une issue. Utiliser le signalement privé GitHub lorsqu'il est activé, sinon demander au mainteneur un canal privé avant de transmettre les détails sensibles. Aucun email ou canal interne non vérifié n'est annoncé. Préciser le SHA, le scénario minimal et l'impact ; masquer les données personnelles et credentials.

## Русский

Приложение локальное, только публичные данные и бумажная симуляция. Dashboard без аутентификации должен оставаться на 127.0.0.1. Проверки Host/Origin, CSP, ограничения запросов и защищённый HTTP/RSS уменьшают риск, но не гарантируют отсутствие уязвимостей. `.env`, БД, журналы и EXE не публикуются. Закреплённые зависимости и аудит требуют проверки результатов на фактическом SHA; ошибки аудита не игнорируются. Windows, реальные AIS-источники, лицензии и локальная модель проверяются отдельно. Для чувствительных отчётов используйте подтверждённый приватный канал, без секретов в публичных issues.

## English

Local public-data monitoring and paper simulation only. The unauthenticated dashboard must remain on 127.0.0.1. Host/Origin checks, CSP, request limits and hardened HTTP/RSS reduce risk; they do not establish complete security. Keep `.env`, databases, logs and executables out of Git. Hashed dependencies and audits must be checked for the actual candidate; audit failures must never be ignored. Windows, live AIS providers, licences and model availability require separate evidence. Use an enabled GitHub private reporting channel or obtain a verified private contact before sharing sensitive details.
