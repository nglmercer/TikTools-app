---
title: "Contrats et découverte de l’API de contrôle"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
translationRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
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
