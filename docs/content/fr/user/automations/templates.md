---
title: "Importer des modèles et gérer les profils"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "49bb4d1eb5dff9f267b30e8d0ee693a6dfcb492ff4609d190370e40c72b094ab"
translationRevision: "49bb4d1eb5dff9f267b30e8d0ee693a6dfcb492ff4609d190370e40c72b094ab"
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
4. Appliquez/importez explicitement et attendez le résultat. Les définitions importées restent désactivées. Sélectionnez-les ensuite et examinez chaque action et événement avant d’activer explicitement ceux souhaités.
5. Commencez par un test limité au journal ou examinez d’abord les définitions. Les tests HTTP, audio, points ou plugins peuvent exécuter des effets.
6. Confirmez le profil actif avant de démarrer le LIVE. Changer de profil modifie le contexte de bibliothèque sélectionné ; ce n’est pas une commande de synchronisation Cloud.

Les paramètres tels que <code v-pre>{{ params.giftName }}</code> sont résolus lors de l’application du modèle. Les références d’exécution telles que <code v-pre>{{ event.user.uniqueId }}</code> restent présentes pour chaque événement, tandis que les [variables globales](variables.md) fournissent des valeurs enregistrées partagées lors de l’exécution.

## Résultat attendu et état {#expected-result-and-status}

Les outils locaux d’importation et d’exportation des modèles/profils sont disponibles. Parcourir Cloud nécessite la capacité correspondante de l’hôte ainsi que le compte/service. Un aperçu de catalogue ne prouve pas que le service externe d’une action fonctionne. Des plugins manquants peuvent rendre les définitions importées inutilisables jusqu’à leur installation et configuration.

## Récupération {#recovery}

Pour du JSON invalide, utilisez les erreurs d’analyse indiquées et le type de document attendu ; renommer un fichier JSON quelconque ne le convertit pas. En cas de conflit de révision, actualisez la bibliothèque enregistrée avant de réappliquer. Conservez le profil fonctionnel exporté et consultez les [sauvegardes](../preferences/backups.md) pour les données que l’exportation de profil n’inclut pas.
