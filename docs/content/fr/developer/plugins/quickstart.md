---
title: "Créer un premier plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "17f8e12522f04bad46b814f437e80c3f480b43331ed1278e49597878e2cee4f8"
translationRevision: "39dd4b7856b0d9a50911669431a9bb7a0e01eb8efe1e0857f3cafc849c44603a"
order: 250
---

# Créer un premier plugin {#build-a-first-plugin}

## Objectif et prérequis {#goal-and-prerequisites}

Produire un paquet validé renvoyant un résumé d’action sans danger. Utilisez le kit `tiktools-plugin` correspondant au dépôt et Bun 1.4.4 pour la création TypeScript. Créez cet exemple dans un répertoire de développement distinct ; c’est un exemple de création, pas une modification des dépendances applicatives.

## Générer et valider {#scaffold-and-validate}

Depuis le dépôt TikTools, compilez la CLI de création avec le lanceur de tâches :

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

Placez le `target/debug/tiktools-plugin` de cette compilation dans votre chemin d’outils, puis, dans le répertoire de création choisi :

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

Le générateur crée un manifeste schema-v3, un identifiant stable préfixé par l’éditeur, un descripteur d’action, un import SDK et une configuration de compilation. Gardez alignés l’environnement `napi-vm`, `apiVersion:1` et l’entrée du module compilé. Le nom est historique ; l’exécution actuelle utilise QuickJS.

## Écrire et compiler le programme invité {#write-and-build-the-guest}

L’action générée peut rester entièrement dans la frontière du résultat typé :

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? 'Bonjour'));
  },
});
```

Installez les dépendances déclarées du paquet généré dans ce paquet distinct, puis lancez `tiktools-plugin build --release`. Il compile JavaScript à l’avance et crée une archive. Il n’y a ni transpileur TypeScript à l’exécution ni processus Node. Le [SDK TypeScript](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts) définit les signatures des utilitaires ; le schéma du manifeste assiste l’éditeur tandis que l’analyseur de l’hôte reste l’autorité.

## Résultat attendu et état {#expected-result-and-status}

`validate` signale les problèmes de compatibilité et de publication ; une compilation réussie crée un paquet `.plugin` avec son manifeste, son entrée et ses métadonnées d’intégrité. Une action limitée à un résumé ne nécessite aucun effet hôte supplémentaire. Si vous ajoutez émission d’événements, HTTP ou audio, mettez à jour les déclarations et examinez l’autorité demandée avant de tester.

## Récupération et vérification {#recovery-and-verification}

Si l’entrée manque, compilez avant d’empaqueter et vérifiez son chemin relatif au manifeste. Ne faites pas pointer l’environnement processus vers du JavaScript non compilé. Pour une préparation locale, utilisez `tiktools-plugin dev stage --into /absolute/path/dev-plugins` et un hôte privé de test configuré pour cette racine. La préparation de développement est distincte de l’installation ; elle ne prouve pas la confiance d’un registre signé. Poursuivez avec les [limites d’exécution](runtimes.md) et la [signature](packaging-registry.md) avant de distribuer cet exemple.

## Détails de fonctionnement {#operational-details}

Lisez `hostCompatibility`, `publishReady` et `warnings` dans `validate`. Une entrée non construite est un avertissement ; `--publish` échoue sur les problèmes de publication, pas tous les avertissements de compatibilité. Revalidez après compilation, examinez entrée/cible empaquetées et testez l’hôte cible. Un succès avant compilation ne prouve pas un paquet utilisable.

Pour Rust, créez `process-rust` ou `native-rust` dans un dossier séparé. Les deux implémentent `Plugin` du SDK et retournent `ActionResult::summary` ; le processus utilise `tiktools_process_plugin!`, le natif `tiktools_export_native_plugin!` et `cdylib`. ABI/confiance natives sont propres à la cible et en processus. Construisez avec la syntaxe des cibles du devkit, gardez le chemin généré et utilisez un hôte privé. Préférez le processus si l’isolation des pannes convient.

```sh
tiktools-plugin new acme.process --template process-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-process
tiktools-plugin new acme.native --template native-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-native
tiktools-plugin build acme-process --release
tiktools-plugin validate acme-process --publish
tiktools-plugin build acme-native --release
tiktools-plugin validate acme-native --publish
```
