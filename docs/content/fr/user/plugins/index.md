---
title: "Installer et gérer les plugins"
docId: "user.plugins.index"
translationKey: "user.plugins.index"
locale: fr
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a7e7179831fc08751d9c6c96c573e518610f7aa6dedddc2e18de2a9fb2a2e3a"
translationRevision: "387c51aa59296dec3944dd3e078bba5f1cbc2f1f6284b88413aa2d6dd1e5991d"
order: 10
---

# Installer et gérer les plugins {#install-and-manage-plugins}

## Objectif et prérequis {#goal-and-prerequisites}

Ajouter une intégration, examiner les accès demandés et confirmer que ses actions sont utilisables. Il faut un paquet de bureau compatible avec le système d’exploitation, l’architecture et la version d’hôte du plugin. Les opérations de catalogue distant et de téléchargement nécessitent un accès réseau ; certains services liés au compte nécessitent aussi une [connexion Cloud](../cloud/account.md).

## Examiner avant d’activer {#review-before-enabling}

1. Ouvrez **Plugins** et examinez Installed, Explore ou Updates. Explore demande uniquement des entrées de plugins. Les modèles et profils d’automatisation se trouvent dans [Automations](../automations/templates.md).
2. Sélectionnez un plugin pour examiner sa description, sa version, sa compatibilité et son environnement d’exécution. Vérifiez l’éditeur et les capacités ou permissions demandées plutôt que de vous fier uniquement à un nom familier.
3. Utilisez le parcours d’installation pour examiner l’aperçu du paquet. Examinez la version exacte, les informations de signature, les accès demandés et les différences avec une version installée avant de confirmer l’installation.
4. Attendez le résultat de la tâche de paquet. Téléchargement, installation et activation sont des étapes distinctes ; une fiche de catalogue seule ne signifie pas que l’environnement d’exécution est prêt.
5. Ouvrez le volet Settings ou Connection du plugin. Fournissez l’adresse de son service et ses identifiants uniquement dans les champs prévus. Utilisez la vérification de connexion déclarée lorsqu’elle est proposée.
6. Activez le plugin, puis vérifiez l’apparition de ses types d’action ou pages. Créez une [règle de journalisation](../automations/first-rule.md) ciblée avant d’attacher une action externe puissante.

Les plugins peuvent exécuter du code natif ou des processus séparés. Une frontière de processus peut isoler un plantage, mais ne constitue pas un bac à sable du système d’exploitation. Installez des logiciels provenant d’éditeurs de confiance. Une signature valide identifie la provenance vérifiée d’un paquet ; elle ne rend pas chaque effet approprié à votre usage.

## Résultat attendu et état {#expected-result-and-status}

L’examen des paquets, les tâches d’installation, les préférences de plugins et la gestion des versions sont disponibles. Les actions réelles dépendent du manifeste installé, des capacités de l’hôte et du service externe. Les processeurs d’événements ont leur propre panneau et peuvent ajouter des champs d’analyse facultatifs aux événements ; les champs bruts restent distincts.

## Récupération {#recovery}

En cas d’échec d’installation, lisez l’erreur de tâche pour identifier un problème de compatibilité, signature, exécution ou téléchargement. N’affaiblissez pas la vérification simplement pour effacer l’erreur. Si une mise à jour pose problème, examinez les versions conservées et utilisez le retour arrière lorsqu’il est disponible. Examinez les automatisations dépendantes avant de désactiver ou désinstaller un plugin. Gardez des [sauvegardes](../preferences/backups.md) des données et configurations des plugins avant de les supprimer.

<TourFrame id="plugin" locale="fr" title="Démo : plugins" />

## Opérations en ligne de commande {#command-line-operations}

Chaque opération de ce guide est aussi disponible depuis la CLI — hors ligne, scriptable et adaptée aux agents. Les lignes d'usage sont la syntaxe du parseur et restent en anglais dans toutes les langues.

<!--@include: ../../../../.generated/cli-domain-plugin.fr.md-->

<!--@include: ../../../../.generated/cli-domain-registry.fr.md-->

<!--@include: ../../../../.generated/cli-domain-processor.fr.md-->

Consultez la [référence complète des commandes](../../developer/cli.md) pour les conventions, la sortie JSON et les marqueurs d’opérations destructives.

<TourFrame id="pluginSetup" locale="fr" title="Guide interactif : Configurer un plugin" />
