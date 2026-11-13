---
title: "Comprendre les classements et compétitions"
docId: "user.points.rankings"
translationKey: "user.points.rankings"
locale: fr
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
translationRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
order: 30
---

# Comprendre les classements et compétitions {#understand-rankings-and-competitions}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir la vue de classement appropriée et reconnaître les limites de l’espace de classements en développement. Une activité entrante est nécessaire pour obtenir des scores pertinents. Commencez par le classement ordinaire de [Points](rules.md) ou les panneaux des meilleurs spectateurs de [Live feed](../live/feed.md) pour simplement examiner les spectateurs actuels.

## Utiliser la bonne vue {#use-the-right-view}

1. Dans **Points**, recherchez un spectateur et triez par points, niveau ou identité. Actualisez avant de comparer un ajustement manuel à une attribution.
2. Dans **Live feed**, examinez les panneaux de classement de spectateurs disponibles. Ils peuvent recevoir des instantanés du moteur de classement indépendamment du tableau des soldes de points.
3. Pour une incrustation, ouvrez [Overlays](../overlays/index.md), sélectionnez le type de classement et examinez les options de source. Une source de classement configurée doit exister avant que sa sortie puisse apparaître.

L’espace dédié **Rankings & competitions** est expérimental. Sa route d’interface est `#/rankings`, mais il est volontairement masqué dans la navigation principale et n’est pas restauré comme onglet de démarrage. L’absence d’entrée latérale ne signifie pas que la route ou le moteur de classement a été supprimé.

## Configurer un classement expérimental {#configure-an-experimental-ranking}

Si votre environnement expose cet espace, commencez avec un créateur, une salle, un regroupement par utilisateur et une fenêtre de session. Ajoutez une métrique annoncée, telle que `tiktok.like` / `like.count`, avec un poids de `1`. Fixez une limite de résultats modeste et créez la définition. Le formulaire prend aussi en charge des fenêtres glissantes et quotidiennes, avec des bornes explicites de durée ou d’événements, ou un décalage de fuseau horaire.

Lisez l’instantané obtenu avant d’introduire d’autres sources. La pause et la reprise affectent la collecte du classement ; la réinitialisation efface intentionnellement son état. Les options de compétition et les sources liées aux votes ont leurs propres règles de regroupement. Ne supposez pas qu’une réinitialisation des points réinitialise aussi une compétition, ni que tous les regroupements personnalisés fonctionnent avec toutes les métriques.

## Résultat attendu et état {#expected-result-and-status}

Les classements ordinaires sont disponibles ; la page de gestion dédiée reste expérimentale. Les scores proviennent de l’hôte et non du comptage des lignes affichées dans le flux. Ce guide ne démontre ni la validation en diffusion réelle/OBS ni celle de toutes les combinaisons de compétition.

## Récupération {#recovery}

Vérifiez le créateur, la salle, l’activation des sources, la fenêtre et l’état de pause lorsque les scores restent vides. Retirez les combinaisons source/regroupement non prises en charge plutôt que d’inventer des noms de champs. Préservez les données importantes par des [sauvegardes](../preferences/backups.md) avant toute réinitialisation, et incluez l’identifiant de définition et le code de diagnostic dans votre rapport.
