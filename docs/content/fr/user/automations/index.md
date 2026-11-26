---
title: "Organiser les automatisations"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "08bb26a20687b58094e908a4e8395abeae3b9adc3932c86d9c3b08f5ebbf8ce3"
translationRevision: "21418d5e43873d05197f9291010893eed584cc38ca18832ec1d3ad859f59c1a4"
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

Toute comparaison accepte une liste de valeurs au lieu d'une seule. La combinaison par défaut est « au moins un » : la condition est remplie quand au moins une valeur correspond. Passez le quantificateur à « tous » quand chaque valeur doit correspondre — avec « n'est pas », cela se lit « n'est aucun de ». Une condition peut aussi nommer un alias de capture : la valeur gagnante est alors disponible pour les modèles d'action sous la forme <code v-pre>{{ match.&lt;alias&gt; }}</code> (ou <code v-pre>{{ match.&lt;position&gt; }}</code> sans alias). Sous les conditions toujours requises, vous pouvez ajouter jusqu'à quatre **alternatives** : des groupes dont toutes les conditions doivent être remplies, et un seul groupe suffit pour déclencher l'événement.

Les tests peuvent exécuter les effets configurés. Lisez la liste d’actions sélectionnée avant d’appuyer sur **Test** ou de déclencher un événement, surtout pour l’audio, HTTP, les points et les opérations de plugins. Utilisez la journalisation pendant l’apprentissage.

## Résultat attendu et état {#expected-result-and-status}

L’édition locale, l’exécution, les modèles, les profils et l’historique sont disponibles. Les déclencheurs de plugins apparaissent uniquement lorsque le plugin correspondant est installé et activé. L’activation dépend de la surface et des valeurs enabled examinées ; vérifiez événements et actions avant confirmation. Le panneau **Changes** de Events/Actions est une [comparaison de référence locale](../cloud/changes.md), pas un téléversement automatique vers Cloud.

## Récupération {#recovery}

En l’absence d’exécution, vérifiez dans cet ordre le profil actif, les états d’activation, le déclencheur, les filtres, le délai et les actions manquantes. En cas de conflit de révision, lisez la définition actuellement enregistrée et examinez-la à nouveau. N’écrasez pas aveuglément la mise à jour d’une autre personne. Exportez un [profil](templates.md) fonctionnel avant des modifications étendues.

## Détails de fonctionnement {#operational-details}

Avant import ou changement de profil, lisez [l’activation par surface](templates.md#operational-details). Le changement peut réactiver des membres désactivés individuellement. Le cooldown est en mémoire, limité à 24 heures ; les spectateurs sans uniqueId partagent un contexte anonyme. À 8 192 clés conservées, la table est vidée : surcharge ou redémarrage peuvent autoriser des exécutions supplémentaires. Ce n’est ni quota durable ni garantie de replay.

Essayez interactivement avant de toucher à votre vraie configuration :

<DemoFrame scenario="automation" locale="fr" panel="automation" title="Essayez l'espace des automatisations" />

## Opérations en ligne de commande {#command-line-operations}

Chaque opération de ce guide est aussi disponible depuis la CLI — hors ligne, scriptable et adaptée aux agents. Les lignes d'usage sont la syntaxe du parseur et restent en anglais dans toutes les langues.

<!--@include: ../../../../.generated/cli-domain-automation.fr.md-->

<!--@include: ../../../../.generated/cli-domain-workflow.fr.md-->

Consultez la [référence complète des commandes](../../developer/cli.md) pour les conventions, la sortie JSON et les marqueurs d’opérations destructives.
