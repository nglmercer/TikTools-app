---
title: "Contrats et découverte de l’API de contrôle"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "a90a2f35314357af0da9a4d0497a9ff8110173eb2939e1c89b487d565bb4eeec"
order: 70
---

# Contrats et découverte de l’API de contrôle {#control-api-contracts-and-discovery}

## Objectif et prérequis {#goal-and-prerequisites}

Appeler le contrat publié par l’hôte sans inventer de noms de méthodes ou schémas de réponse. Utilisez un hôte privé actif et la CLI correspondante. Le [registre Rust](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) canonique décrit les méthodes ; les modules d’enregistrement fournissent leurs schémas, capacités, exigences de bureau et indicateurs de concurrence. Découvrez la compilation réelle avant de choisir une opération.

## Examiner, puis appeler {#inspect-then-call}

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

La dernière commande utilise un environnement de vérification isolé. Elle ne valide pas une connexion LIVE réelle. Pour stdio/IPC local, le protocole utilise du JSON délimité par lignes ; WSS distant transporte un message par trame et exige d’abord `collaboration.hello`.

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

Une réponse reprend l’identifiant et contient `result` ou `error`. Les erreurs utilisent une chaîne `code`, un message et des `data` sûres facultatives ; ne supposez pas les codes d’erreur numériques d’un autre serveur JSON-RPC. Les notifications utilisent `method:"event"` et `params:{topic,data}`. Séparez les notifications de la résolution des réponses en attente.

## Préserver les métadonnées {#preserve-metadata}

Les appels versionnés peuvent porter `meta.expectedRevision` au niveau supérieur. Les appels durables lient aussi un identifiant de commande et une instance d’hôte selon la découverte. Lisez `meta.resourceVersions`, `meta.command` et `meta.hostInstanceId` avec le résultat métier. Les révisions sont des chaînes décimales, pas des nombres JavaScript. L’utilitaire `callWithMeta` de l’interface dans `src/web/shared/api/control-contracts.ts` préserve cette enveloppe.

## Résultat attendu et état {#expected-result-and-status}

Une méthode découverte est prise en charge par cette compilation d’hôte ; les permissions distantes et restrictions de ressources s’appliquent toujours. Les méthodes de propriétaire local ne sont pas implicitement des méthodes distantes. Les requêtes génériques conservent le même parcours d’opération que les enveloppes SDK typées. Les références techniques partagées doivent dériver du registre/des schémas plutôt que d’un second tableau de méthodes maintenu manuellement.

## Récupération {#recovery}

Pour `method_not_found`, comparez la découverte et les versions client/hôte. Pour `revision_conflict`, conservez la proposition et comparez les versions actuelles. Un délai de transport dépassé peut laisser une mutation acceptée en cours ; interrogez son identifiant de commande original conservé lorsque cela est pris en charge. Ne créez jamais un nouvel identifiant simplement pour contourner l’incertitude. Poursuivez avec les [agents](agents.md) ou la [récupération distante](remote-client.md).

## Référence versionnée des méthodes {#method-reference}

Générée depuis l’instantané de découverte versionné dans le dépôt ; l’hôte actif reste l’autorité.

<!--@include: ../../../.generated/control-api.fr.md-->

## Détails de fonctionnement {#operational-details}

Utilisez un `TIKTOOLS_HOME` jetable, retirez les variables héritées selon [stockage](storage.md), démarrez `tiktools host --ipc` et utilisez la même racine dans le terminal de contrôle. La recette suppose le workflow jetable `docs-example` ; créez d’abord l’exemple désactivé d’[automatisation](automations.md#operational-details). Conservez révision de `meta.resourceVersions` et `meta.hostInstanceId` de la lecture ; remplacez `REVISION`/`HOST_INSTANCE`. Gardez un ID stable avec sa charge exacte. Après timeout, consultez l’ID original puis actualisez la ressource ; `accepted`, `running`, `unknown` n’autorisent pas un nouvel effet. Un conflit impose une nouvelle lecture/revue, pas un écrasement automatique. `points.adjust` est additif : ID de commande admis, sans révision attendue.

`--include-meta` conserve résultat et métadonnées ; la sortie ordinaire extrait le résultat. `--dry-run --explain` valide/planifie sans exécuter ; explications sur stderr, JSON sur stdout. Les appels destructifs exigent `--confirm`. La sortie masque les secrets reconnus ; `--secrets-visible` change seulement le rendu CLI et ne retrouve pas les valeurs cachées par l’hôte. Relisez manuellement. `api events` limite durée/nombre et filtre les sujets. Discovery reste l’autorité sur features/permissions ; les descriptions générées gardent le texte exact du registre.

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
