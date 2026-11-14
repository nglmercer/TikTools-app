---
title: "Architecture de l’application et des services"
docId: "developer.architecture"
translationKey: "developer.architecture"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "00ea88ebd11714484d717b7d7533d29158afd498121db362a3d300bc21889501"
translationRevision: "c1d4d9160416178f0dc2412de9b2c62dec536ceedcca7cef41297fc3a2809b81"
order: 30
---

# Architecture de l’application et des services {#application-and-service-architecture}

## Objectif et prérequis {#goal-and-prerequisites}

Placer une modification dans le service propriétaire de son état. Lisez la [configuration](setup.md) et comprenez la différence entre un hôte existant et un hôte autonome explicite. Le bureau construit un `AppCore`, l’enveloppe dans un `ControlApi` et expose cette instance à la WebView, à la CLI locale et aux agents.

## Suivre une opération {#trace-an-operation}

1. Trouvez l’enregistrement de la méthode dans `crates/tiktools-control-api/src/modules/`. Il fournit les schémas, la politique et les métadonnées de découverte.
2. Suivez le gestionnaire dans `crates/tiktools-core/src/control/` et le service propriétaire. Placez ici la validation métier et les mutations persistantes plutôt que dans un adaptateur de transport.
3. Ajoutez l’enveloppe typée dans `crates/tiktools-client` ; son test de parité compare la couverture du SDK à la surface enregistrée.
4. Consommez la méthode via `src/web/shared/api/control-client.ts`. Les entités Vue conservent l’état de présentation ; le bureau ne transmet à la WebView ni base de données ni objet d’intégration native.

Winit possède la boucle d’événements, Wry la WebView et l’intégration de zone de notification son icône. `HostEmitter` et les événements métier transportent des valeurs sérialisées hors du cœur. Le bureau transmet le travail via le proxy de sa boucle d’événements ; l’évaluation de scripts WebView reste sur ce thread. Tokio exécute les opérations asynchrones de l’hôte.

## Vérification concrète des frontières {#concrete-boundary-check}

```sh
bun scripts/task.ts test web control-client
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
```

La première commande vérifie le client de l’interface. La seconde exécute directement la cible d’intégration du SDK en préservant le fichier de verrouillage ; les options de cible Cargo passent par `rust` plutôt que par la grammaire plus restreinte des tests ciblés.

## Résultat attendu et état {#expected-result-and-status}

Tous les clients observent le même parcours faisant autorité pour les opérations et événements. `host --ipc`, `host --stdio` et `--standalone` créent intentionnellement un propriétaire sans interface graphique ; les commandes CLI normales se rattachent à un propriétaire actif. L’ancien parcours `PageMessage`/`IpcRouter` reste du code de compatibilité et ne doit pas recevoir de nouvelles variantes de messages.

## Récupération {#recovery}

Si deux clients divergent, comparez l’identité de l’hôte, le répertoire d’application, les fonctionnalités de compilation et la découverte avant d’ajouter un autre store. Un plugin absent est un problème de catalogue d’exécution, pas une raison de compiler son implémentation dans le cœur. Les documents d’architecture historiques contiennent des chemins d’interface retirés ; suivez le [cœur actuel](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/lib.rs) et le [plan de l’interface](frontend.md) lorsqu’ils diffèrent.

![AppCore TikTools, transport, ControlClient et espace de travail Vue réel](/architecture.svg)

## Détails de fonctionnement {#operational-details}

Les features Cargo du binaire choisi déterminent les chemins compilés. Le cœur active par défaut `native-plugins` et `napi-vm-node-api` ; `persistence` fournit SQLite, `native-tiktok` l’adaptateur/entrée LIVE, et `plugin-install`, `cloud-auth`, `app-update` leurs services. Consultez `cargo tree -p tiktools-desktop -e features` (ou la CLI choisie), puis `system.info`/discovery. La compilation ne prouve pas la disponibilité configurée. Testez l’entrée LIVE avec la régression `native-tiktok` de [performance](performance.md).
