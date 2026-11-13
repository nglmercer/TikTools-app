---
title: "Consulter les statistiques sur plusieurs dates"
docId: "user.points.statistics"
translationKey: "user.points.statistics"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a7333d75baa9aec6e0c4afb69d1c773575a56ee8da982f0fa5a11a73b903ec2d"
translationRevision: "c7716c33d0e7770701883e19c813694b26c7931bc4f388146508d67a89ac3aeb"
order: 20
---

# Consulter les statistiques sur plusieurs dates {#review-statistics-across-dates}

## Objectif et prérequis {#goal-and-prerequisites}

Comparer l’activité d’un créateur sur une période pertinente et comprendre pourquoi les totaux diffèrent du flux visible ou du solde de points. Il faut de l’activité enregistrée dans l’installation locale et le bon créateur sélectionné via [Connections](../live/connections.md). La connexion Cloud n’est pas la source de ces statistiques locales.

## Interroger la période enregistrée {#query-the-recorded-period}

1. Ouvrez **Analytics** et vérifiez le créateur affiché à côté des commandes de dates. Le créateur sélectionné détermine les agrégats stockés qui seront interrogés.
2. Choisissez **Today**, `7d`, `30d`, `90d` ou une période personnalisée. Pour des dates personnalisées, fixez explicitement le début et la fin et vérifiez leur ordre.
3. Lisez la période affichée et le nombre de sessions avant de comparer les résultats. Les limites des dates locales utilisent un décalage de fuseau horaire ; changer le fuseau de l’ordinateur peut modifier l’interprétation d’une période.
4. Sélectionnez la métrique nécessaire : discussions, cadeaux, mentions J’aime, diamants ou pic de spectateurs. Comparez les périodes avec les graphiques d’activité et examinez les meilleurs spectateurs renvoyés dans le tableau des contributeurs.
5. Recherchez dans ce tableau un spectateur déjà présent dans les données renvoyées. Il ne s’agit pas d’une recherche illimitée de tous les spectateurs jamais observés.
6. Sélectionnez **Refresh** après une nouvelle activité ou à votre retour sur la page. Notez le créateur, les dates et la métrique avec tout chiffre partagé.

La page actuelle appelle l’opération native `analytics.summary` avec un créateur et une période. Elle lit des agrégats persistants, y compris les sessions enregistrées précédemment ; elle n’est pas limitée aux événements encore conservés dans le flux actuel. La base analytique stocke des agrégats plutôt qu’une transcription des discussions.

## Résultat attendu et état {#expected-result-and-status}

Les statistiques locales persistantes et les requêtes par période sont disponibles. Des dates sans activité renvoient légitimement un résultat vide. Les soldes de points peuvent différer car les [taux d’attribution](rules.md), ajustements manuels et récompenses d’automatisation représentent une mesure différente. Diamants, nombres de cadeaux et points ne sont pas des unités interchangeables.

## Récupération {#recovery}

Si les données semblent absentes, effacez la recherche de contributeur, élargissez la période, confirmez le créateur et actualisez. Après une erreur de l’hôte, conservez son code de diagnostic et réessayez une fois la connexion rétablie. Effacer le flux ne supprime pas les statistiques. Un nouveau répertoire de données n’a aucun historique antérieur ; utilisez des [sauvegardes](../preferences/backups.md) pour déplacer une installation.

<DemoFrame scenario="analytics" locale="fr" title="Démo : statistiques" />

## Détails de fonctionnement {#operational-details}

Analytics demande actuellement les 10 principaux contributeurs pour le créateur et les dates choisis. La recherche filtre ce sous-ensemble, sans rechercher tous les spectateurs. Ces agrégats persistants ont une période et une unité différentes des diamants/likes de la session et des soldes de points. Conservez créateur, dates et mesure lors des exports et comparaisons.
