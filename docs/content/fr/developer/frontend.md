---
title: "Couches de l’interface et circulation des données"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
translationRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
order: 40
---

# Couches de l’interface et circulation des données {#frontend-layers-and-data-flow}

## Objectif et prérequis {#goal-and-prerequisites}

Étendre l’application Vue sans coupler une page à l’IPC natif. L’interface utilise Vue 3.5, Pinia 4, Vue Router 5 et vue-i18n 11 selon les plages de l’espace de travail installé. Utilisez les dépendances verrouillées et les commandes de [configuration](setup.md) plutôt qu’un nouveau squelette d’interface.

## Trouver la couche responsable {#find-the-owning-layer}

L’organisation Feature-Sliced Design actuelle se trouve sous `src/web/` :

| Couche | Responsabilité | Exemple existant |
| --- | --- | --- |
| `app` | Initialisation, routage, structure et durée de vie de l’exécution | `app/model/use-app-runtime.ts` |
| `pages` | Composition des routes | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | Panneaux composés substantiels | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | Opérations utilisateur et leurs brouillons | `features/apply-rule-template/` |
| `entities` | Stores métier et logique métier réutilisable | `entities/rule-template/` |
| `shared` | API, primitives d’interface, i18n et utilitaires génériques | `shared/api/control-client.ts` |

Composez les couches supérieures à partir des inférieures. Gardez les appels hôte derrière le client de contrôle partagé ; le choix du transport appartient à l’initialisation. Les anciens emplacements `src/web/platform` et `src/web/views` des articles précédents sont historiques.

## Implémenter une modification ciblée {#implement-a-focused-change}

1. Repérez la route dans `app/router/routes.ts`, puis suivez sa page vers la fonctionnalité et le store d’entité.
2. Utilisez un composant partagé existant pour les champs, états ou boîtes de dialogue. Préservez ses libellés, son comportement de focus et son contrat d’événements.
3. Séparez les propositions modifiables des enregistrements sauvegardés de l’hôte. Utilisez `shared/lib/request-scope.ts` pour ignorer les réponses asynchrones obsolètes ; l’invalidation n’annule pas une mutation acceptée par l’hôte.
4. Ajoutez les clés de langue via `shared/i18n` et libérez les abonnements à la fin de leur durée de vie propriétaire.

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## Résultat attendu, état et récupération {#expected-result-status-and-recovery}

La route affiche les données actuelles de l’hôte et conserve les modifications non enregistrées en cas de conflit. Rankings dispose d’un code source et d’une route mais reste masqué dans la navigation normale ; ne déduisez pas la disponibilité produit de la présence d’un composant. Les initialisations native, navigateur local et démo peuvent fournir des transports différents tout en utilisant le même code de fonctionnalité.

Si une réponse obsolète remplace une requête plus récente, vérifiez sa génération avant de modifier le serveur. Si une route perd sa page de plugin, revenez à la vue des plugins installés comme le fait déjà l’exécution. Le [runtime de l’application](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) est la référence pour le rattachement, la resynchronisation et le nettoyage.
