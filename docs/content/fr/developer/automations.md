---
title: "Exécution des automatisations et contrats de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "57cc664e4ef2ff26fafb31df6458da439f075c673fbb3e06e635fa03c1032f42"
translationRevision: "419a220f5757155a6730637608349020f3437af2747f159ab0165080cf04d74a"
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

## Détails de fonctionnement {#operational-details}

Le graphe suivant utilise les ports v1 trigger → transform → log du catalogue et reste désactivé. Enregistrez-le comme `docs-graph.json` et vérifiez versions/ports avec `automation.nodes.list`. `workflow save --record` persiste ; `workflow get` et enable/disable avec métadonnées vérifient l’état sauvegardé. Utilisez les révisions fraîchement lues. Sauvegarder/activer ne prouve pas un exécuteur complet ou un éditeur visuel sur toute build : le runtime normal de ce checkout exécute Events/Actions de comportement. Utilisez le modèle de log de [modèles](templates.md#operational-details), révisez-le, activez volontairement les deux enregistrements et testez un chat nettoyé pour un résultat inoffensif. Sauvegarder un graphe ne prouve pas l’exécution LIVE.

Pour l’enrichissement, installez/activez TextIntel avec `events.enrich`, vérifiez état/métriques dans Processors et testez l’échantillon. Inspectez `event.intel.providers.textintel.comment.moderation.blocked` avant un filtre is-true vers `core.log` ; l’absence du champ n’est pas une preuve négative. Circuit ouvert ou saturation peuvent ignorer les processeurs. L’échantillon ne valide pas un LIVE complet.

Pour un travail incertain, consultez `automation.library.get`, `automation.definition.get`, `automation.adoption.get` ou `automation.approval.get`. Conservez pending.commandId, identité hôte et candidat. Résolvez l’opération originale (`*.resolve`) ; si unknown, examinez l’état réel avant `*.review_unknown` avec ID original et reconnaissance explicite. L’adoption exige une nouvelle revue autorité/dépendances, pas seulement inspect/preview. `automation.effects.get {effectId}` inspecte un effet externe retenu : aucune autorisation d’envoi ni reprise aveugle. Conservez les inconnus pour réconciliation opérateur.

```json
{"schemaVersion":1,"id":"docs-example","name":"Docs","enabled":false,"nodes":[{"id":"start","type":"trigger.event","version":1,"position":{"x":0,"y":0},"config":{"eventType":"tiktok.chat"}},{"id":"text","type":"transform.template","version":1,"position":{"x":200,"y":0},"config":{"template":"docs-example"}},{"id":"log","type":"action.log","version":1,"position":{"x":400,"y":0},"config":{"message":"docs-example"}}],"edges":[{"id":"a","kind":"flow","source":"start","sourcePort":"flow","target":"text","targetPort":"flow"},{"id":"b","kind":"flow","source":"text","sourcePort":"flow","target":"log","targetPort":"flow"}]}
```

```sh
tiktools api call automation.nodes.list '{}' --json
tiktools workflow save --record "$(cat docs-graph.json)"
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.enable '{"id":"docs-example"}' --expected-revision REVISION --dry-run --json
tiktools api call automation.library.get '{}' --json
tiktools api call automation.adoption.get '{}' --json
```
