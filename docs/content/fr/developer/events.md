---
title: "Normalisation, lots et lacunes d’événements"
docId: "developer.events"
translationKey: "developer.events"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
translationRevision: "77fbd47a8e56f8384eb3cd7015f10db4b5c35678d4e1fb59eab078ec62c79064"
order: 100
---

# Normalisation, lots et lacunes d’événements {#event-normalization-batches-and-gaps}

## Objectif et prérequis {#goal-and-prerequisites}

Consommer les événements LIVE sans double comptage ni confusion entre récupération de présentation et exécution. Comprenez la [persistance](storage.md) et utilisez des fixtures expurgées plutôt que des captures de comptes réels. `tiktools-tiktok` enveloppe les dépendances verrouillées du signataire/décodeur et exporte des valeurs TikTools stables ; le cœur n’expose pas les objets protobuf comme API applicative.

## Suivre le pipeline implémenté {#follow-the-implemented-pipeline}

L’entrée capture le contexte original de connexion créateur/salle avant le travail asynchrone. La normalisation produit les valeurs de discussion, cadeau, mention J’aime, activité sociale et cycle de vie. Le cœur publie les originaux et gère séparément la comptabilisation/les effets. Les processeurs peuvent attacher des éléments dérivés bornés sous `event.intel` ; les échecs préservent les événements bruts avec des diagnostics dégradés. Les contrats d’automatisation générés décrivent uniquement les champs émis.

L’identité de rejeu serveur comprend créateur, salle, méthode, type et identifiant de message non nul. Des identifiants manquants ne permettent pas d’établir un rejeu. La comptabilisation des séries de cadeaux utilise l’identité émetteur/cadeau/groupe et des maxima cumulés, afin qu’un paquet final ne récompense pas deux fois la même quantité. Les prix inconnus restent à zéro. Les transitions d’agrégats peuvent permettre un franchissement de seuil avec `previous < threshold <= total` ; elles ne remplacent pas les originaux publics.

## Implémenter un consommateur {#implement-a-consumer}

1. Utilisez les types de `src/automation/contracts/generated/` ou l’analyseur d’enveloppe métier des widgets à cette frontière.
2. Préservez les curseurs de livraison lors de la sérialisation et dédupliquez les chevauchements en réunissant temps réel et rattrapage.
3. Traitez les lots ordonnés sans déclencher un rendu réactif par événement.
4. Exposez des signaux explicites de lacune/surcharge. Récupérez la présentation avec des pages bornées ou un nouvel instantané, jamais en invoquant à nouveau les points ou actions.

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

La [source des contrats Rust](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts) génère le registre des événements, schémas et échantillons ; ne reproduisez pas manuellement des tableaux complets de champs.

## Résultat attendu et état {#expected-result-and-status}

Le chevauchement est dédupliqué seulement tant que l’événement identifié reste dans le stockage fini actuel. Les files, compteurs, historiques et caches de rejeu restent bornés. Il ne s’agit ni d’une archive illimitée d’événements ni d’une garantie universelle d’exécution unique. L’épuisement de capacité et la persistance indisponible ont des conséquences explicites sur l’état de santé.

## Récupérer et vérifier {#recover-and-verify}

Utilisez des fixtures expurgées de cadeaux/émoticônes et testez les paquets de fin de série, identifiants manquants, identités entre salles et lacunes de reconnexion. Consultez la révision du signataire dans le `Cargo.toml` actuel plutôt que de copier un hachage d’un ancien rapport de fiabilité. Les méthodes distantes inconnues restent inconnues jusqu’à leur ajout délibéré dans la normalisation et les contrats générés.

## Détails de fonctionnement {#operational-details}

Les valeurs actuelles (2026-11-13) sont des limites d’implémentation, pas un SLA. L’entrée LIVE contient 2 048 éléments et la file d’effets 128 ; ce ne sont pas des budgets d’octets. Le cycle de vie attend une place. Cadeaux/follows/shares attendent au plus une seconde puis sont retirés avec signalement de lacune ; chat/likes/joins/autres données sont retirés immédiatement en saturation. Consultez `droppedCritical`, pertes de lots/unités de likes et d’effets : une entrée perdue peut n’avoir jamais exécuté ses effets. Le backfill répare uniquement l’affichage, jamais points ou actions.

La déduplication conserve au plus 4 096 identités pendant 300 secondes en mémoire du processus, entre reconnexions tant que ce stockage existe. Expiration, éviction, ID absent ou redémarrage retirent cette protection. La protection contre le chevauchement reste limitée à ce cadre.

`ui.feed.backfill` reçoit `{version:1,after:0,limit:64}`, avec curseur numérique global et limite 1–512. Consultez `oldestCursor`, `latestCursor`, `gap`, `truncated` et continuez depuis le dernier curseur reçu. `ui.feed.creator` utilise `{creatorId,streamId,after,limit}`, des curseurs textuels par flux/créateur, une limite 1–64 et un budget d’octets. Ne mélangez pas les curseurs. Flux changé/évincé, curseur futur, lacune d’entrée ou `CreatorFeedUnavailable` imposent une actualisation de la présentation et un signalement d’historique incomplet, jamais une reconstruction des effets.

TikTok initialise session, salle, catalogue de cadeaux, bundle de signature puis socket. Configuration : `TIKTOOLS_TIKTOK_SIGNING_BUNDLE`, `TIKTOOLS_TIKTOK_BUNDLE_CACHE`, `TIKTOOLS_TIKTOK_BUNDLE_URL`. Valeurs : cinq tentatives, délai initial 2 s plafonné à 30 s. HTTP 429 respecte Retry-After ou un minimum croissant 30–300 s avec jitter, au plus deux reprises pour limitation, avec cooldown conservé après annulation/épuisement. Les autres reprises exigent une panne de transport transitoire. Inspectez `Reconnecting { attempt, delay_ms }` et l’étape, sans partager cookies ni signatures privées.

```sh
tiktools api call system.health '{}' --json
```
