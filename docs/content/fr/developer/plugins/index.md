---
title: "Présentation du développement de plugins"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
order: 240
---

# Présentation du développement de plugins {#plugin-development-overview}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir la plus petite frontière de plugin répondant à votre intégration. TikTools découvre les paquets d’exécution depuis les racines intégrées, utilisateur et de développement ; ajouter un paquet ne nécessite pas de recompiler un plugin nommé dans l’hôte. Utilisez le kit de développement/SDK correspondant et un profil privé de développement avant de tester des effets.

## Suivre le parcours de création {#follow-the-authoring-path}

1. Commencez par le [démarrage rapide](quickstart.md) pour générer, valider et compiler un paquet isolé.
2. Choisissez un [environnement d’exécution](runtimes.md) selon les dépendances natives et la confiance, pas seulement l’extension du fichier.
3. Déclarez les [permissions et l’interface](permissions-ui.md) pour que l’hôte puisse examiner les effets et afficher la configuration.
4. Consultez l’[empaquetage et le registre](packaging-registry.md) pour l’intégrité, la confiance de l’éditeur et la politique de mise à jour.

L’autorité Rust de bas niveau est `crates/tiktools-plugin-api` : validation du manifeste, protocole et contrats d’interface. `crates/tiktools-plugin-sdk` fournit l’interface de création Rust ; `packages/plugin-sdk` contient les utilitaires TypeScript et le [schéma pour l’éditeur](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json). Le chargement à l’exécution appartient à `crates/tiktools-plugin-loader`, pas à l’interface.

## Comprendre les contributions {#understand-contributions}

Une action demande un effet via un résultat typé. Les sources d’événements publient des déclencheurs ; les observateurs reçoivent des enveloppes métier stables `{topic,data}` avec livraison bornée. Les processeurs ajoutent des éléments sans effet sous `event.intel`, avec échéances, fusion déterministe et isolation des défaillances. Les paramètres/actions déclaratifs permettent à l’hôte d’afficher une configuration ordinaire. Une page compilée personnalisée utilise un courtier de plugin restreint dans son propre cadre/fenêtre ; son script ne fait jamais partie du runtime Vue principal.

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

Ce sont des vérifications ciblées hôte/SDK, pas la validation d’un paquet externe arbitraire.

## Résultat attendu et état {#expected-result-and-status}

Les parcours natif, processus, déclaratif et `napi-vm` fondé sur QuickJS sont implémentés. WASM est un emplacement d’exécution facultatif sans moteur général actif dans les compilations normales. Les intégrations d’exemple se trouvent dans le [dépôt distinct plugins-example-tiktools](https://github.com/nglmercer/plugins-example-tiktools) ; les noms de sources dans les anciens documents n’en font pas des fonctions produit embarquées.

## Récupération {#recovery}

En cas d’échec de découverte, examinez la compatibilité du manifeste, la priorité des racines, la cible et le chemin d’entrée. Pour une intention refusée, examinez les capacités déclarées et accordées au lieu de contourner l’hôte. Un démarrage incertain doit être résolu avant remplacement. Les frontières natives/processus conservent les limites de confiance du système ; le nom de l’environnement ne prouve jamais l’existence d’un bac à sable.
