# Pravda-Bot — livraison pour utilisation privée

**Statut** : application Windows de préproduction **non signée**, remise pour utilisation privée, sans distribution commerciale. La remise n'est pas un GO de publication entreprise.

## Périmètre décidé pour cette remise

Le client a demandé la remise de l'application sans certificat Authenticode ni signature de distribution, avec l'audit technique du code et de la CI réalisé avant remise, puis les tests sur machines virtuelles propres après réception. Ces contraintes ne doivent pas empêcher la remise d'un candidat privé clairement identifié. Elles restent des limites de validation, et non des validations effectuées.

- Application GUI native Tauri 2 / Rust. Exécutables Windows et installateurs MSI/NSIS produits par CI.
- Aucun environnement Python ou Node.js nécessaire pour utiliser l'application installée ; ces outils sont réservés aux tests et au build.
- Flux publics observés explicitement attribués à leur source ; l'absence de cotation réelle doit être signalée. Portefeuille de simulation séparé et étiqueté.
- **Trading réel et retraits désactivés** faute d'autorisation distincte. Le portefeuille papier ne doit pas contacter d'API privée d'ordres.
- **Updater automatique désactivé** : aucune installation de mise à jour non signée ; ne pas basculer vers un updater artisanal.
- Pas de certificat de signature de production supposé, aucune demande de PFX ni secret d'utilisateur. Les avertissements éventuels de Windows concernant l'absence de signature ne doivent pas être contournés par la désactivation de Defender ou SmartScreen.

## Paquet privé et vérification avant lancement

Utiliser le paquet **pravda-windows-single-file-setup** téléchargé à partir d'une exécution GitHub Actions liée au SHA exact de la branche de livraison, avec l'empreinte SHA-256 jointe à la remise. Vérifier le hachage dans PowerShell :

```powershell
(Get-FileHash -LiteralPath .\pravda-setup.exe -Algorithm SHA256).Hash.ToLowerInvariant()
```

Comparer la valeur au manifeste de livraison obtenu séparément ; interrompre l'installation si elle diffère. Sur Windows, la commande `pravda-setup.exe --verify-only` teste l'intégrité du payload avant installation sans installer le programme. L'installateur reste non signé ; ne pas supprimer les protections du système pour le lancer. Le test final du logiciel sur l'ordinateur du destinataire n'a pas été effectué dans GitHub.

## Preuves déjà produites et limites

Les workflows Rust, RustSec, CodeQL, SonarCloud, WebView2 et packaging doivent tous être vérifiés sur le même commit que l'artefact. La recette CI installe et désinstalle MSI et NSIS sur un runner Windows déjà doté de WebView2, et teste la GUI et l'authentification. Ce résultat ne prouve **pas** l'installation hors ligne de WebView2 sur deux VM vierges.

Après livraison, les essais VM devront reprendre le même exécutable, vérifier l'absence de WebView2 au départ, couper la connectivité au niveau hyperviseur, conserver les rapports distincts et comparer les empreintes. Aucune attestation fictive ou signature de production ne sera générée.

## Condition de non-régression

Sur un événement GitHub pull_request, `GITHUB_SHA` peut être le commit de fusion temporaire. Tous les rapports GUI installés doivent utiliser `PRAVDA_SOURCE_SHA` défini par le HEAD de la PR. Les rapports qui contiennent le SHA temporaire sont des preuves historiques de test GUI mais ne sont **pas** des attestations exact-SHA du build, et doivent être remplacés lors de la prochaine exécution.

Références : [PR #172](https://github.com/suvorovpravda/Pravda-Bot/pull/172) ; [registre #160](https://github.com/suvorovpravda/Pravda-Bot/issues/160).
