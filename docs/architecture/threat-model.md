# Modèle de menace — Desktop Windows Pravda-Bot

**État :** RFC PR-01, revue documentaire ; basé sur `main@423ab09c85228466d90a23269d903db3415df0c6`. Aucune exploitation n'est prétendue.

## Actifs à protéger

- Vérificateur de mot de passe local : `%LOCALAPPDATA%/Pravda-Bot/pravda_auth.json`, aucune transmission à des tiers.
- SQLite : historique des cotations (`pravda.db` et WAL), paramètres futurs et état d'analyse ; la simulation papier actuelle est **en mémoire**, ne pas promettre sa durabilité.
- Données marché et fraîcheur : BTC/ETH/SOL, Kraken/Binance, statut live/stale, contrôle de spread et frais fixes à 0,36 %.
- Intégrité des commandes reçues depuis WebView2 ; absence d'exécution réelle d'ordres de trading.
- Binaires et MSI, ressources front-end embarquées, identité éditeur, provenance et contenu du package.
- Logs de diagnostic : ne doivent révéler ni mot de passe, cookie, jeton de session, clés, chemins sensibles inutiles ni données de l'utilisateur non essentielles.

## Adversaires / frontières

| Menace | Capacité supposée | Frontière de confiance |
|---|---|---|
| Page Web distante hostile | Créer des requêtes, tenter navigation/fetch/CSRF, images ou chargement distant | WebView2 ↔ code Rust ; ACL Tauri ; aucun IPC depuis origine distante |
| HTML/JS local compromis (XSS, supply chain) | Exécuter du JS dans la WebView | Capacité minimale + validation/rate limits côté commande |
| Processus local non privilégié du même utilisateur | Connexion à des ports loopback, lecture/écriture selon ACL du compte, attaques de fichiers | Retrait du HTTP du desktop ; permissions fichiers ; pas d'exécution arbitraire |
| Autre compte Windows | Accès aux fichiers selon ACL/identité du système | Dossier profil utilisateur, ACL, aucune élévation de privilèges par défaut |
| Artefact de distribution altéré | Remplacement de .exe/.msi/scripts avant exécution | Authenticode, chaîne de certificats, hash manifeste et source de distribution |
| Compromission CI ou dépendance | Modifier code de build, exfiltrer secret | Permission minimale GitHub Actions, pinning/review, séparation construction/signature, attestation |
| Panne / arrêt pendant écriture | Crash, corruption, perte de session ou historique | SQLite WAL, migrations atomiques, tests de restart/rollback |
| Flux de données falsifiés ou périmés | Valeurs anormales/malformées de marché | Validations finies, fraîcheur, horodatage, spread / fees ; fail closed |
| Malware du même utilisateur | Contrôle significatif du compte et de la session | **Hors garantie de protection totale** du mot de passe applicatif |

## Énoncés STRIDE et défenses requises

| STRIDE | Scénario | Mesure attendue | Preuve |
|---|---|---|---|
| Spoofing | Fenêtre secondaire/ou origine non approuvée appelle `auth_login` | Autorité Tauri par fenêtre + manifeste strict + autorisation côté Rust | Tests appel refusé depuis fenêtre non listée |
| Tampering | Fichier de vérificateur ou base modifié | ACL profil, ouverture validée, migration transactionnelle, gestion corruption sans écrasement | Tests fichier invalide / absence de remplacement |
| Repudiation | Acte privilégié impossible à tracer | événements structurés avec horodatage, sans valeurs sensibles | tests redaction |
| Information disclosure | Logs ou réponses IPC exposent hash/jeton | DTO sans secret, erreurs publiques, interdiction de logging d'arguments sensibles | scans de logs et tests DTO |
| Denial of service | Flux tick envahit IPC ou requêtes historique non bornées | coalescing, queue bornée, throttling, requêtes SQLite plafonnées | tests burst/slow consumer |
| Elevation of privilege | Script .cmd/.ps1 modifié, commande `shell` ou `fs` trop permissive | aucun script dans MSI, pas de shell IPC, listes de commandes explicites, signature | scan contenu MSI / matrice ACL |

## Risques spécifiques constatés dans le prototype

1. La WebView charge `http://127.0.0.1:8080` et l'auth utilise POST `/setup` : des changements de contrôles `Sec-Fetch-*` ont provoqué des blocages **observés** sur Windows. Ne pas confondre blocage UX avec contournement exploitable.
2. Le frontend utilise un script inline et la CSP HTTP actuelle inclut `script-src 'unsafe-inline'` : objectif cible sans code inline pour réduire les possibilités d'exécution en cas d'injection.
3. `tools/Demarrer-Pravda-Bot.cmd` lance PowerShell avec `-ExecutionPolicy Bypass`. Ce fallback est séparé du binaire Tauri ; il doit rester **hors MSI**.
4. Aucun bundling/signature d'éditeur n'est défini sur la configuration de production auditée. Une empreinte SHA-256 seule n'authentifie pas l'éditeur si un attaquant peut remplacer le manifeste.
5. `pravda-server.exe` de diagnostic reste un binaire réseau ; toute distribution doit préciser les protections et ne pas le fournir comme alternative de production.

## Règles de non-régression

- Auth : configuration obligatoire avant accès privilégié ; récupération d'une session interdite après logout/expiration/redémarrage ; test complet en Windows réel.
- Toute commande IPC lit l'identité de l'invocateur et vérifie son droit métier avant effet ; une capacité insuffisante ne constitue jamais une preuve que le métier est autorisé.
- Toute mutation persistante est atomique, sans réinitialisation silencieuse du vérificateur précédent.
- Aucun `shell:default`, `fs:default`, liste de chemins arbitraires ni permission « tout autoriser ».
- Aucun chargement JavaScript distant ; code externe explicitement revu avant introduction.
- CI de la release doit empêcher tout binaire non signé, cert incorrect ou horodatage manquant.

## Références

- NIST SP 800-207 Zero Trust Architecture : https://csrc.nist.gov/pubs/sp/800/207/final
- Tauri Runtime Authority : https://v2.tauri.app/security/runtime-authority/
- Tauri custom ACL permissions : https://v2.tauri.app/reference/acl/permission/
