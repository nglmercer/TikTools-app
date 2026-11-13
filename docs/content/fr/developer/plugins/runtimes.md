---
title: "Environnements d’exécution des plugins et limites de confiance"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
translationRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
order: 260
---

# Environnements d’exécution des plugins et limites de confiance {#plugin-runtimes-and-trust-limits}

## Objectif et prérequis {#goal-and-prerequisites}

Choisir un format exécutable dont les dépendances et la confiance correspondent à l’hôte. Lisez la [présentation des plugins](index.md), validez le manifeste contre la compilation réelle et examinez les restrictions propres aux cibles avant de charger du code. La compatibilité du protocole, celle de l’ABI native et les permissions accordées sont des vérifications indépendantes.

## Choisir un environnement implémenté {#choose-an-implemented-runtime}

| Environnement | Exécution | Confiance et portabilité |
| --- | --- | --- |
| `native` | ABI C via `libloading` | Code de confiance dans le processus ; propre à la cible |
| `process` | Exécutable autonome, stdin/stdout encadrés | Isolation des plantages, permissions système ordinaires de l’utilisateur |
| `napi-vm` | JS compilé à l’avance sur QuickJS via `rquickjs` | Effets hôte refusés par défaut ; les extensions natives facultatives changent la confiance |
| `declarative` | Descripteurs d’intégration validés | Aucun programme invité ; les effets passent par le courtier hôte |
| `wasm` | Emplacement d’exécution facultatif | Moteur général non activé dans les compilations normales |

L’ABI native v1 échange des octets sérialisés via une petite table C ; les conteneurs Rust, objets de trait et futures ne la traversent pas. Les bibliothèques natives restent chargées jusqu’à l’arrêt. L’encadrement des processus utilise une longueur u32 petit-boutiste suivie d’octets JSON, pas des RPC de contrôle délimitées par lignes. Gardez les sorties diagnostiques hors du canal stdout encadré.

## Comprendre la compatibilité QuickJS {#understand-quickjs-compatibility}

Le nom de manifeste `napi-vm` reste stable, mais les anciennes affirmations sur un interpréteur distinct en Rust pur sont historiques. Les programmes invités reçoivent les variables globales de compatibilité documentées, pas un environnement Node complet. `napi_get_uv_event_loop` n’est pas pris en charge. Les extensions natives `.node` utilisent un parcours dans le processus volontairement de confiance, limité par liste autorisée et vérifié par SHA-256 ; le nom du runtime JavaScript ne les rend pas sûres.

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

Utilisez des tests ciblés du chargeur lorsqu’une fixture complète d’extension native nécessiterait des outils de plateforme indisponibles.

## Résultat attendu et état {#expected-result-and-status}

L’analyse du manifeste rejette les combinaisons protocole/ABI non prises en charge avant le chargement. Android autorise QuickJS portable et les entrées de processus Android/Bionic valides, mais rejette les bibliothèques dynamiques natives et extensions `.node`. Une ressource de bureau Linux/glibc n’est pas un plugin processus Android.

## Récupération {#recovery}

Redémarrez un hôte privé après remplacement d’une bibliothèque native. Si le démarrage dépasse le délai et devient incertain, le gestionnaire peut conserver l’identité du worker et bloquer le remplacement ; ne contournez pas cet état en déclarant l’arrêt réussi. Les tampons d’événements sont bornés et la livraison peut signaler du retard ; l’émission au démarrage n’est pas une file durable. L’[implémentation du chargeur](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) définit le moteur actuel et le comportement du cycle de vie.
