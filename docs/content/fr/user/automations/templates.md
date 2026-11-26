---
title: "Importer des modèles et gérer les profils"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "74217892395ddea29ed42ae4105097d815949c11029e1598a32cd2e978963f11"
translationRevision: "fa2bc77eeeb4ab8a49d2e3144865a58ab1b3f3fd894d58782fdd3b64c6daa36c"
order: 50
---

# Importer des modèles et gérer les profils {#import-templates-and-manage-profiles}

## Objectif et prérequis {#goal-and-prerequisites}

Réutiliser une règle ou un ensemble de règles examinées sans activer d’effets accidentellement. Il faut **Automations**, un fichier JSON ou une entrée de catalogue de confiance, ainsi que les plugins requis par ses actions. Exportez le profil actuel avant de remplacer une configuration établie.

## Choisir le bon outil {#choose-the-right-surface}

Un document `.tiktemplate.json` décrit un événement et ses actions. Un document `.tikprofile.json` regroupe plusieurs modèles. Ouvrez la boîte de dialogue des modèles depuis Automations pour les modèles intégrés/personnalisés, sa bibliothèque Cloud ou l’importation manuelle de modèles. Utilisez la barre d’outils des profils pour leur création, sélection, exportation et importation manuelle. **Plugins → Explore** parcourt les plugins ; les modèles et profils se trouvent ici.

## Examiner et importer {#review-and-import}

1. Ouvrez la boîte de dialogue d’importation voulue et choisissez un fichier ou collez du JSON pris en charge. Vérifiez le nom préparé et la source avant de poursuivre.
2. Pour un modèle, examinez le déclencheur, les types d’action et les paramètres. Pour un profil, examinez aussi le nombre d’entrées et les paramètres communs.
3. Remplacez les destinations d’exemple, chemins de médias et identifiants de spectateurs par des valeurs adaptées à votre configuration. Ne collez pas d’identifiants secrets dans un fichier destiné au partage.
4. Appliquez/importez explicitement et attendez le résultat. Ne supposez pas les définitions désactivées : examinez les valeurs enabled. Sélectionnez-les ensuite et examinez chaque action et événement avant d’activer explicitement ceux souhaités.
5. Commencez par un test limité au journal ou examinez d’abord les définitions. Les tests HTTP, audio, points ou plugins peuvent exécuter des effets.
6. Avant de confirmer un profil, examinez l’activation de ses membres et la désactivation des autres, y compris éléments isolés et membres précédemment désactivés. Les effets exigent déclencheurs/conditions ; ce n’est pas un sync Cloud.

Les paramètres tels que <code v-pre>{{ params.giftName }}</code> sont résolus lors de l’application du modèle. Les références d’exécution telles que <code v-pre>{{ event.user.uniqueId }}</code> restent présentes pour chaque événement, tandis que les [variables globales](variables.md) fournissent des valeurs enregistrées partagées lors de l’exécution.

## Résultat attendu et état {#expected-result-and-status}

Les outils locaux d’importation et d’exportation des modèles/profils sont disponibles. Parcourir Cloud nécessite la capacité correspondante de l’hôte ainsi que le compte/service. Un aperçu de catalogue ne prouve pas que le service externe d’une action fonctionne. Des plugins manquants peuvent rendre les définitions importées inutilisables jusqu’à leur installation et configuration.

## Récupération {#recovery}

Pour du JSON invalide, utilisez les erreurs d’analyse indiquées et le type de document attendu ; renommer un fichier JSON quelconque ne le convertit pas. En cas de conflit de révision, actualisez la bibliothèque enregistrée avant de réappliquer. Conservez le profil fonctionnel exporté et consultez les [sauvegardes](../preferences/backups.md) pour les données que l’exportation de profil n’inclut pas.

## Détails de fonctionnement {#operational-details}

L’import manuel de modèle enregistre d’abord un document de catalogue ; son application crée actions/événements liés. Le parseur utilise true si `enabled` manque et l’application conserve ces valeurs. L’import manuel de profil prépare ses entrées pour revue propriétaire. Le catalogue Cloud stocke des modèles ; un profil Cloud prépare une mutation avec provenance. Ni origine ni aperçu réussi ne garantissent une désactivation universelle. Examinez comparaison finale et états enregistrés. En CLI, utilisez explicitement `tiktools template import file.tiktemplate.json --disabled` ou `tiktools template profile-import file.tikprofile.json --disabled`.

Changer de profil modifie l’activation : la confirmation active les membres du profil cible et désactive toutes les actions/événements hors profil, y compris les éléments isolés. Des membres précédemment désactivés peuvent être activés. Vérifiez les deux listes. Les effets exigent encore déclencheur, conditions et runtime ; le changement n’envoie pas de synchronisation Cloud à lui seul.

Essayez interactivement avant de toucher à votre vraie configuration :

<DemoFrame scenario="automation" locale="fr" panel="automation" title="Essayez l'espace des automatisations" />

## Opérations en ligne de commande {#command-line-operations}

Chaque opération de ce guide est aussi disponible depuis la CLI — hors ligne, scriptable et adaptée aux agents. Les lignes d'usage sont la syntaxe du parseur et restent en anglais dans toutes les langues.

<!--@include: ../../../../.generated/cli-domain-template.fr.md-->

Consultez la [référence complète des commandes](../../developer/cli.md) pour les conventions, la sortie JSON et les marqueurs d’opérations destructives.
