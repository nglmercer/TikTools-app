---
title: "Persistance et récupération durable"
docId: "developer.storage"
translationKey: "developer.storage"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
translationRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
order: 60
---

# Persistance et récupération durable {#persistence-and-durable-recovery}

## Objectif et prérequis {#goal-and-prerequisites}

Modifier l’état persistant via son service propriétaire et préserver les résultats récupérables. Utilisez un profil jetable, fermez son hôte avant de copier des bases et lisez les [métadonnées de contrôle](control-api.md) avant de modifier des ressources versionnées. `TIKTOOLS_HOME` sélectionne la racine de l’application ; `AppPaths` détermine ses données et stockages privés.

## Identifier le stockage {#identify-the-store}

`crates/tiktools-core/src/paths.rs` définit `data/tiktok-points.db`, `data/tiktok-automation.db` et `data/tiktok-analytics.db`. Les modules de service SQLite possèdent leurs schémas. Points/spectateurs et définitions d’automatisation ont des registres métier distincts ; l’analytique est une base séparée, omise par les anciens schémas à deux bases.

Au démarrage, les anciennes bases locales au dépôt peuvent être copiées vers les données applicatives de la plateforme uniquement si la destination est absente. Une destination existante n’est jamais écrasée. Les définitions JSON restent lisibles pendant les changements d’exécution native. N’implémentez pas une migration en supprimant le profil d’un utilisateur ou en réécrivant les tables depuis Vue.

## Effectuer un changement récupérable {#make-a-recoverable-change}

1. Repérez la transaction du service et ses tests de schéma/migration.
2. Stockez le changement métier, la révision et la réservation de journal/outbox requise dans la même transaction lorsque le domaine le permet.
3. Préservez les marqueurs de suppression des révisions. Un identifiant d’enregistrement réutilisé ne doit pas accepter une révision d’édition antérieure à sa suppression.
4. Publiez les métadonnées d’invalidation validées via l’outbox existante ; une nouvelle tentative de livraison ne doit pas répéter l’action sous-jacente.
5. Rouvrez le même répertoire de test privé après interruption et résolvez la commande originale avant une autre mutation.

Vérifications utiles en lecture seule sur un hôte de test déjà actif :

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## Résultat attendu et état {#expected-result-and-status}

Les journaux durables pris en charge des workflows, points et propriétaires conservent les résultats originaux après redémarrage. Un reçu décrit sa commande historique ; lisez séparément l’état actuel. Les stockages des espaces Cloud ajoutent des partitions origine/compte et des vérifications de génération ; ils ne remplacent pas l’autorité SQLite locale. Tous les effets externes ou domaines n’ont pas la même couverture de récupération durable.

## Récupération {#recovery}

Conservez ensemble les fichiers SQLite et tout WAL/SHM actif lors d’une sauvegarde hors ligne. Un délai d’interface dépassé ne prouve pas une annulation. Pour `revision_conflict`, préservez le brouillon et comparez une nouvelle lecture. Pour des effets externes incertains, examinez l’enregistrement d’effet conservé plutôt que de rejouer un flux. Consultez la [référence des chemins](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs) et la [récupération Cloud](cloud.md) pour leurs frontières distinctes.
