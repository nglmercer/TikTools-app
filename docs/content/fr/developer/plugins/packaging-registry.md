---
title: "Empaquetage, signature et registres de plugins"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b5c1b8dc63aafc5216467a00317c6670ac6744235abfc724fe372f3a28427ee8"
translationRevision: "f559c829e08049810b7cfeef756f1668a775134b579b4ec228443252af3da023"
order: 280
---

# Empaquetage, signature et registres de plugins {#plugin-packaging-signing-and-registries}

## Objectif et prérequis {#goal-and-prerequisites}

Distribuer un paquet dont les octets, l’autorité de l’éditeur et la compatibilité de l’hôte peuvent être vérifiés indépendamment. Commencez avec un plugin validé et une cible prise en charge. La signature nécessite des clés privées d’auteur conservées hors du dépôt ; la délégation racine du registre est une responsabilité distincte de l’opérateur.

## Préparer une archive à examiner {#prepare-a-reviewable-archive}

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` compile le paquet et crée des archives `.plugin`. L’entrée du manifeste, les ressources, langues, sorties d’interface et dépendances d’exécution pertinentes sont incluses avec `checksums.json`. Les compilations natives/processus nécessitent une ressource cible compatible ; les paquets uniquement JavaScript restent portables jusqu’à ce que des extensions natives ou fonctions de plateforme les contraignent.

Pour une archive compilée existante, utilisez des chemins de fichiers exacts plutôt qu’un joker non examiné :

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

Remplacez le nom de l’archive par la sortie réelle propre à la cible de compilation. La signature couvre les sommes de contrôle, qui couvrent les fichiers du paquet. Les clés publiques peuvent être partagées ; les fichiers de clés privées ne doivent entrer ni dans le contrôle de version ni dans les journaux.

## Confiance et politique de mise à jour {#trust-and-update-policy}

Le protocole du registre statique possède une autorité signée par la racine et des clés déléguées d’index/éditeur. L’approbation de l’espace de noms d’un éditeur est distincte de l’intégrité du paquet, et une empreinte du catalogue Cloud ne remplace aucune des deux chaînes de signature. Le résolveur vérifie canal, moteur, schéma/protocole/ABI, environnement d’exécution et cible. Les versions retirées, révoquées ou incompatibles restent inéligibles même lorsque leur hachage correspond.

L’installation par l’hôte présente d’abord un examen. Les mises à jour automatiques nécessitent une politique autorisée, un paquet non épinglé, un éditeur signé compatible et aucune nouvelle autorité ; des permissions élargies exigent un examen. L’installation depuis le catalogue Cloud du produit est distincte de la création de registre statique de bas niveau, et les parcours produit ordinaires rejettent les URL de paquets distants arbitraires.

## Résultat attendu, récupération et vérification {#expected-result-recovery-and-verification}

L’installateur peut vérifier les octets avant d’appliquer un paquet examiné et conserver un état de retour arrière/récupération. La préparation de développement échappe à la gestion de l’installateur. En cas d’échec de confiance, réparez la délégation ou les métadonnées de l’éditeur plutôt que d’utiliser normalement des options de contournement non signé. En cas de démarrage incertain, arrêtez le remplacement et résolvez le résultat conservé.

Utilisez `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` et les tests du registre pour les fixtures hors ligne. Le [vérificateur de paquets](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src) et le [modèle du registre](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs) définissent les formats techniques ; l’infrastructure de publication nécessite une validation distincte de l’opérateur.

## Détails de fonctionnement {#operational-details}

Interprétez le JSON de vérification, pas seulement le code de sortie. `verify` peut réussir avec `signature.status` égal à `unsigned` ou `untrusted`. N’acceptez que `verified` avec ID de paquet, éditeur et clé attendus, puis vérifiez la politique de distribution. Une clé locale `--trust` prouve cette signature locale, pas la délégation du registre ni le namespace. Les checksums seuls ne prouvent ni autorité ni confiance updater. L’automatisation ci-dessous doit échouer sur tout résultat inattendu.

```sh
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json > verification.json
bun -e 'const r = await Bun.file("verification.json").json(); if (r.signature?.status !== "verified" || r.id !== "acme.hello" || r.signature.publisher !== "acme" || r.signature.keyId !== "acme-1") throw new Error("Package trust did not match policy");'
```
