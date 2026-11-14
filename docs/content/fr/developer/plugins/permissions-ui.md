---
title: "Permissions des plugins et interface isolée"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "240142875d416f523b6daee5f53bf37c16b4b15f409561b38bd5357668239f50"
translationRevision: "221ac73a7c62f3c8996bfd69451d4ff435ddb2b9ab5f31cae10c03273b09211b"
order: 270
---

# Permissions des plugins et interface isolée {#plugin-permissions-and-isolated-ui}

## Objectif et prérequis {#goal-and-prerequisites}

Exposer la configuration et des effets hôte intentionnels sans importer le script du plugin dans l’application principale. Utilisez un manifeste validé, le paquet SDK/interface du plugin correspondant et un hôte de test privé. Lisez la [confiance des environnements](runtimes.md) : une capacité de manifeste n’est pas un bac à sable système pour les bibliothèques natives ou exécutables de processus.

## Déclarer une contribution restreinte {#declare-a-narrow-contribution}

Décrivez les champs d’action/paramètres par des schémas JSON et indications d’interface dans le manifeste. Les pages déclaratives simples affichent des composants hôte de confiance. `events.publish` conditionne les intentions d’événements ; l’audio hôte nécessite en plus la capacité `audio.play` et la permission `audio.output`. Consultez le [schéma canonique du manifeste](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) et l’analyseur Rust plutôt que de copier une liste exhaustive de permissions dans la documentation.

Les processeurs nécessitent leur contribution d’enrichissement déclarée et renvoient des annotations/vues/journaux sans effet. Les observateurs métier nécessitent `events.subscribe` et reçoivent une enveloppe stable par livraison bornée, pas l’énumération Rust interne des événements ou des accès de transport arbitraires.

## Construire une page personnalisée isolée {#build-an-isolated-custom-page}

Un `ui` de manifeste avec `apiVersion:1` et `mode:"webview"` pointe vers les ressources compilées de `ui/dist/`. L’application principale héberge un cadre à origine opaque autorisant les scripts ; les fenêtres détachées facultatives utilisent un courtier natif restreint. Le script personnalisé ne s’exécute jamais dans le runtime Vue principal.

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

L’hôte fournit l’identité du plugin. Le client ne peut pas choisir un autre identifiant de plugin. Les méthodes du courtier sont limitées à ses propres paramètres/actions/options/événements ainsi qu’à la langue/au thème. Une page ouverte sans courtier échoue explicitement. Exécuter une action depuis une page produit un effet réel, même si le bouton indique un test.

## Résultat attendu et état {#expected-result-and-status}

Le rendu déclaratif, les cadres intégrés et les fenêtres natives détachées disposent de contrats et tests implémentés. CORS/CSP, la liaison à la fenêtre source et les listes autorisées du courtier natif appliquent ensemble cette frontière ; le Shadow DOM ne le fait pas. La propriété IPC non configurable de WebKitGTK n’a pas besoin d’être supprimée car le gestionnaire natif lui-même est restreint.

## Récupérer et vérifier {#recover-and-verify}

Pour un cadre vide, examinez les chemins des ressources, la CSP et le CORS d’origine opaque avant d’assouplir la politique. Pour `broker unavailable`, vérifiez le contexte d’hébergement. Exécutez `bun scripts/task.ts test bun plugin-ui` et `bun scripts/task.ts test web plugin-webview-host`. Le [paquet du courtier](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts) définit le comportement d’enveloppe/version et la sémantique de nettoyage.

## Détails de fonctionnement {#operational-details}

Les processeurs exigent `processorTypes` et la capacité `events.enrich` ; les paquets déclaratifs sont ignorés car ils n’exécutent pas de processeur. Installez/activez le paquet et vérifiez son état avant l’échantillon. Partez du manifeste TextIntel et de `Plugin::enrich` du SDK, pas d’un manifeste seulement déclaratif. Voir [la recette opérationnelle](../automations.md#operational-details).

Les appels avec promesse échouent sans broker, mais `subscribe` en page supérieure retourne une fonction vide et les erreurs d’abonnement iframe sont ignorées. Vérifiez `broker.transport()` puis un appel tel que `getSettings()` et affichez son rejet ; un abonnement ne prouve pas la connexion. Vérifiez l’isolation avec la commande exacte ci-dessous.

```sh
bun scripts/task.ts test web plugin-webview-host
```

```ts
const broker = createBrokerClient();
console.log(broker.transport());
await broker.getSettings();
```

```json
{"capabilities":["events.enrich"],"processorTypes":[{"id":"acme.analyze","title":{"default":"Analyze"},"eventTypes":["tiktok.chat"],"stage":"pre-filter","timeoutMs":250,"failureMode":"pass-through","inputs":[{"path":"event.data.comment","role":"message"}]}]}
```
