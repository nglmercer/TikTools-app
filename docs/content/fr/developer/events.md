---
title: "Normalisation, lots et lacunes d’événements"
docId: "developer.events"
translationKey: "developer.events"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
translationRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
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

Le chevauchement de reconnexion ne duplique ni les livraisons affichées ni la comptabilisation protégée contre le rejeu prise en charge. Les files, compteurs, historiques et caches de rejeu restent bornés. Il ne s’agit ni d’une archive illimitée d’événements ni d’une garantie universelle d’exécution unique. L’épuisement de capacité et la persistance indisponible ont des conséquences explicites sur l’état de santé.

## Récupérer et vérifier {#recover-and-verify}

Utilisez des fixtures expurgées de cadeaux/émoticônes et testez les paquets de fin de série, identifiants manquants, identités entre salles et lacunes de reconnexion. Consultez la révision du signataire dans le `Cargo.toml` actuel plutôt que de copier un hachage d’un ancien rapport de fiabilité. Les méthodes distantes inconnues restent inconnues jusqu’à leur ajout délibéré dans la normalisation et les contrats générés.
