---
title: "Questions fréquentes"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
order: 40
---

# Questions fréquentes {#frequently-asked-questions}

## Objectif et prérequis {#goal-and-prerequisites}

Résoudre les confusions courantes d’utilisation avant de modifier des données ou identifiants. L’application de bureau doit être lancée pour les vérifications ci-dessous ; les [prérequis](../requirements.md) décrivent les paquets pris en charge. Les fonctions locales présentées sont disponibles sauf mention d’une limite expérimentale ou dépendante d’un service.

## La connexion Cloud connecte-t-elle TikTok ? {#does-cloud-sign-in-connect-tiktok}

Non. Ouvrez [Connections](../live/connections.md), choisissez un créateur et démarrez le LIVE séparément. L’identité Cloud sert aux services liés au compte. Un cookie TikTok doit uniquement être saisi dans le champ de connexion masqué prévu.

## Pourquoi un flux connecté est-il vide ? {#why-is-a-connected-feed-empty}

Une salle calme peut n’avoir aucun nouvel événement. Choisissez tous les événements, effacez la recherche et reprenez le défilement, puis examinez l’état de connexion. Suspendre le défilement laisse l’ingestion et les automatisations fonctionner. Effacer le flux ne réinitialise ni les points ni les statistiques persistantes.

## Les statistiques concernent-elles uniquement cette session ? {#are-statistics-only-for-this-session}

La [page Analytics](../points/statistics.md) actuelle interroge des agrégats persistants pour le créateur et la période sélectionnés. Élargissez les dates et actualisez pour consulter l’historique enregistré. Ce n’est pas une transcription complète des discussions. Les soldes de points suivent d’autres règles et peuvent inclure des ajustements manuels ou d’automatisation.

## Où se trouvent les modèles, variables et classements ? {#where-are-templates-variables-and-rankings}

Les modèles et profils se trouvent dans [Automations](../automations/templates.md) ; Plugins Explore demande des plugins. Les valeurs d’exécution se trouvent dans Automations Variables. L’espace expérimental Rankings conserve `#/rankings`, mais il est masqué dans la navigation principale et la restauration au démarrage. Les classements de points ordinaires et les panneaux de classement du flux restent des vues distinctes.

## L’enregistrement synchronise-t-il tout ? {#does-saving-synchronize-everything}

Non. Events/Actions Changes compare une référence locale. L’enregistrement des Variables reste local, avec des opérations distinctes d’envoi/adoption/retrait avec examen lorsqu’elles sont prises en charge. Les comparaisons de secrets sont masquées. Les incrustations Cloud utilisent création/révision et bibliothèque ; cette version n’a pas d’adaptateur Changes partagé pour les incrustations. Lisez [Changes](../cloud/changes.md) avant de choisir une direction.

## Les tests sont-ils sans effet, et les importations s’exécutent-elles immédiatement ? {#are-tests-harmless-and-do-imports-run-immediately}

Les tests peuvent exécuter des effets, notamment de l’audio, des requêtes réseau et des modifications de points. Commencez par [core.log](../automations/first-rule.md). Les définitions importées restent désactivées ; examinez-les et activez-les explicitement. Installer un plugin nécessite aussi de configurer son service avant que ses actions puissent fonctionner.

## Résultat attendu et récupération {#expected-result-and-recovery}

Ces vérifications doivent identifier la page responsable sans réinitialisation. Les recherches automatiques de mises à jour sont facultatives ; l’installation est expérimentale et désactivée par les conditions de publication actuelles. La collaboration distante complète et la recherche de pseudonymes en production dépendent encore du déploiement et de la validation. Si le doute persiste, préservez vos données par des [sauvegardes](backups.md) et recueillez un [rapport de diagnostic](troubleshooting.md) avec version, heure et étapes de reproduction.
