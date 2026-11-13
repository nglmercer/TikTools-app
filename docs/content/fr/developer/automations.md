---
title: "Exécution des automatisations et contrats de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
translationRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
order: 150
---

# Exécution des automatisations et contrats de scripts {#automation-execution-and-script-contracts}

## Objectif et prérequis {#goal-and-prerequisites}

Implémenter une automatisation à partir des descripteurs de l’hôte et des contrats d’événements générés. Travaillez dans un profil privé avec les effets externes désactivés jusqu’à un test intentionnel. Comprenez la [normalisation des événements](events.md) et les [métadonnées de contrôle](control-api.md) ; l’éditeur Vue configure l’exécution possédée par Rust.

## Construire une règle minimale {#build-a-minimal-rule}

Une action de comportement stocke un type d’action configuré. Un événement de comportement sélectionne un déclencheur, des filtres, un délai et des identifiants d’action. Les graphes de workflow composent plutôt des nœuds et ports versionnés. Les descripteurs intégrés proviennent du cœur ; ceux des plugins proviennent des manifestes installés. Ne codez jamais en dur un second catalogue d’actions dans l’éditeur.

1. Sélectionnez un déclencheur émis dans `src/automation/contracts/generated/event-registry.generated.ts`.
2. Utilisez son échantillon pour choisir un vrai chemin de champ pointé tel que `event.user.uniqueId` ou `event.data.giftName`.
3. Commencez par la journalisation ou une action désactivée, puis testez le filtre et le délai avec un événement expurgé.
4. Enregistrez via le coordinateur natif propriétaire, en conservant la proposition de l’éditeur et les versions examinées lorsque cela est requis.

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

Le [répertoire des contrats générés](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated) possède les types d’événements de l’interface, les métadonnées du registre et les échantillons. Étendez les contrats Rust et régénérez ensemble lorsque vous exposez un autre champ.

## Frontière des scripts {#script-boundary}

`core.code` et les nœuds de script des workflows exécutent QuickJS avec des limites. Ils reçoivent les valeurs JSON `event`, `inputs` et `data` (alias de inputs). Aucun chargeur Node, système de fichiers, réseau, WebView ou objet de base de données n’est fourni. Gardez les transformations déterministes et sérialisables en JSON ; utilisez des actions hôte configurées pour HTTP, les points, l’audio ou les effets de plugins. Les limites source/résultat et budgets d’exécution appartiennent à l’hôte, pas à un délai du navigateur.

## Résultat attendu et état {#expected-result-and-status}

Les règles enregistrées s’exécutent via le runtime du cœur et son courtier de capacités. Les processeurs peuvent ajouter des éléments `event.intel` sans remplacer les champs bruts. Automations Changes est un outil d’examen local, pas une synchronisation de configuration à l’échelle du compte. Variables avec examen dispose d’une vraie implémentation native ; Overlay Changes est un travail ultérieur distinct non fusionné.

## Récupération {#recovery}

En cas de conflit, comparez les enregistrements actuels sans perdre la proposition. Un brouillon distant reste désactivé en attendant l’examen du propriétaire et le transfert d’autorité ; l’aperçu ou l’inspection du manifeste seul n’autorise pas l’exécution. Les résultats d’effets externes inconnus nécessitent le parcours d’inspection/récupération des effets conservés. Rejouer l’historique d’interface ne doit jamais déclencher à nouveau l’automatisation. Utilisez les [modèles](templates.md) pour une importation contrôlée de bibliothèque et les [tests](testing.md) pour des preuves propres à chaque frontière.
