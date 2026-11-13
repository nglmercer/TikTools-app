---
title: "Organiser les automatisations"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
translationRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
order: 10
---

# Organiser les automatisations {#organize-automations}

## Objectif et prérequis {#goal-and-prerequisites}

Construire une petite bibliothèque compréhensible qui réagit aux événements LIVE. Il faut un hôte de bureau fonctionnel. Une véritable [connexion LIVE](../live/connections.md) est nécessaire pour valider les événements distants ; les actions de plugins nécessitent aussi un plugin et un service prêts.

## Comprendre la bibliothèque {#understand-the-library}

Une **Action** est une opération configurée réutilisable, comme écrire une ligne de journal ou jouer un son. Un **Event** choisit un déclencheur, des filtres, un délai et des actions liées. Un profil regroupe les définitions d’un usage ; un seul ensemble de profil est actif à la fois. Les [Variables](variables.md) d’exécution fournissent des valeurs réutilisables par plusieurs actions.

1. Ouvrez **Automations** et confirmez le profil actif avant toute modification. Recherchez séparément dans Events et Actions pour savoir quel type d’enregistrement vous sélectionnez.
2. Commencez par la [première règle de journalisation](first-rule.md). Créez son action, puis un événement qui la référence.
3. Choisissez un déclencheur pris en charge par l’hôte actuel. Utilisez le sélecteur de champs pour ajouter des filtres adaptés. Plusieurs conditions restreignent les correspondances ; elles ne modifient pas la charge utile entrante.
4. Définissez un délai et sa portée. Un délai global limite la règle pour tous les spectateurs ; un délai par utilisateur suit chaque spectateur séparément.
5. Reliez les actions voulues dans l’ordre. Le mode normal exécute la liste choisie ; le mode aléatoire y sélectionne une action. Une action de délai reporte les actions suivantes sans ralentir l’ingestion LIVE.
6. Enregistrez, examinez les changements de définition affichés, puis activez explicitement les enregistrements à exécuter. Consultez l’historique pour le nom de l’action, son résultat et son erreur.

Les tests peuvent exécuter les effets configurés. Lisez la liste d’actions sélectionnée avant d’appuyer sur **Test** ou de déclencher un événement, surtout pour l’audio, HTTP, les points et les opérations de plugins. Utilisez la journalisation pendant l’apprentissage.

## Résultat attendu et état {#expected-result-and-status}

L’édition locale, l’exécution, les modèles, les profils et l’historique sont disponibles. Les déclencheurs de plugins apparaissent uniquement lorsque le plugin correspondant est installé et activé. Les définitions importées restent désactivées jusqu’à une activation explicite. Le panneau **Changes** de Events/Actions est une [comparaison de référence locale](../cloud/changes.md), pas un téléversement automatique vers Cloud.

## Récupération {#recovery}

En l’absence d’exécution, vérifiez dans cet ordre le profil actif, les états d’activation, le déclencheur, les filtres, le délai et les actions manquantes. En cas de conflit de révision, lisez la définition actuellement enregistrée et examinez-la à nouveau. N’écrasez pas aveuglément la mise à jour d’une autre personne. Exportez un [profil](templates.md) fonctionnel avant des modifications étendues.
