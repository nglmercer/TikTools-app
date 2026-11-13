---
title: "Examiner les changements par domaine"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f875a22e66dd822d7ea1344188d8b7b087a23a777e072240a4a3c7c57969f995"
translationRevision: "eaf4ca846ad1f45e40a499c945c0d8c1b150f1c027a4d840ecfc1bb831773d9c"
order: 20
---

# Examiner les changements par domaine {#review-changes-by-area}

## Objectif et prérequis {#goal-and-prerequisites}

Comprendre ce qu’un panneau **Changes** compare avant de réinitialiser, envoyer ou adopter des données. Il faut du contenu local enregistré. Les opérations Cloud nécessitent en plus un hôte pris en charge, le mode de synchronisation approprié et un [compte connecté](account.md). Les panneaux portant le même nom n’utilisent pas tous le même transport.

## Examiner Events et Actions localement {#review-events-and-actions-locally}

1. Ouvrez **Automations**, sélectionnez Events ou Actions et ouvrez **Changes**.
2. Lisez l’avis : ce domaine compare les définitions actuelles à une référence conservée sur cet appareil. La première visite chargée initialise cette référence ; il est donc normal qu’elle puisse n’afficher aucun changement.
3. Sélectionnez un enregistrement ajouté, modifié ou supprimé et examinez les valeurs avant/après.
4. Choisissez **Mark as reviewed** uniquement pour faire de la section actuelle la nouvelle référence locale. Cela ne téléverse pas les définitions et n’approuve pas l’exécution déléguée.
5. Utilisez **Reset** uniquement pour restaurer la référence. Pour un nouvel enregistrement, la réinitialisation le supprime ; pour un enregistrement modifié/supprimé, elle restaure la définition conservée. Enregistrez d’abord une exportation si nécessaire.

L’adaptateur Events/Actions n’a pas de collection Cloud pour les définitions individuelles dans cette version. Une comparaison sans changements n’est ni une sauvegarde Cloud ni la preuve qu’un autre ordinateur a reçu vos modifications.

## Examiner Variables et Plugins explicitement {#review-variables-and-plugins-explicitly}

Variables dispose de son propre partage natif avec examen : enregistrez localement, récupérez les propositions, examinez la comparaison et envoyez, appliquez/adoptez ou retirez explicitement. Les valeurs secrètes ou bloquées restent masquées et ne peuvent pas être envoyées par ce parcours. L’enregistrement local ne téléverse jamais. Consultez [Variables](../automations/variables.md).

Plugin Changes utilise son espace de synchronisation pris en charge pour les sélections de plugins et les paramètres préparés. La publication s’effectue par section plutôt que par envoi séparé pour chaque ligne. Les paramètres affichent les noms de clés modifiées sans exposer leurs valeurs ; les conflits permettent de choisir la version locale ou Cloud, sans fusion arbitraire des champs. Lisez l’action de section et le mode de synchronisation sélectionné avant de publier.

Les incrustations Cloud utilisent le parcours de création/révision de la [bibliothèque et de l’éditeur](../overlays/index.md). Aucun adaptateur Changes partagé pour les incrustations n’est disponible dans cette version de base.

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

La comparaison locale des automatisations et le partage natif conditionné aux capacités sont disponibles dans leurs domaines respectifs. Les opérations produisant des effets distants dépendent du service ; leur validation en production n’est pas implicite. Si un examen devient obsolète, actualisez et comparez à nouveau. Si le résultat n’est pas confirmé, consultez l’état ou le journal avant de créer une autre opération. Un délai d’attente dépassé ne prouve jamais qu’aucun changement n’a eu lieu.

<DemoFrame scenario="cloud" locale="fr" tour="changes" title="Démo : Cloud" />

## Détails de fonctionnement {#operational-details}

Avant préparation/migration v1 → v2, sauvegardez et coordonnez les versions compatibles sur tous les appareils. Dans setup, préparez, vérifiez la compatibilité et prévisualisez ; examinez sélections, attentes, conflits et éléments uniquement v1 sans les supposer synchronisés. Exiger l’en-tête d’époque est irréversible pour les écritures v2 du compte et peut refuser les anciens appareils. Un timeout de barrière est ambigu ; la répéter avance encore l’époque. Actualisez état époque/migration et aperçu avant d’agir. Lire l’époque ne prouve pas la fin des anciennes écritures.

La récupération dépend du mode : Changes Events/Actions compare des références locales sans écriture Cloud. Pour Variables/Plugins révisés, rechargez les comparaisons périmées et vérifiez révision, compte et inventaires. Si des plugins manquent, révisez/réparez l’inventaire setup puis recréez un aperçu. Pour une intention incertaine, conservez ID/preuves et consultez historique/état ; la récupération technique exige une nouvelle revue des entrées figées et de la révision Cloud. Reprendre cette intention n’envoie pas une écriture distante à lui seul. Arrêtez si la réconciliation reste impossible, sans mutation de remplacement.
