---
title: "Contrats Cloud, synchronisation et examen"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: fr
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "06ae46545d1366d4f456c78a0e6583ea0338afbd496467e4d25b3abe1a5430fb"
translationRevision: "2cc81ecc7fd36c99e129ac480ec73e673e79ccf136a296143e2d670fa0e7dbd0"
order: 180
---

# Contrats Cloud, synchronisation et examen {#cloud-contracts-synchronization-and-review}

## Objectif et prérequis {#goal-and-prerequisites}

Développer les fonctionnalités de configuration de compte et de catalogue pour l’instance Cloud provisionnée. Le fonctionnement uniquement local reste valide. Pour le travail authentifié, utilisez un compte et un serveur de test dédiés ainsi que le transfert de session natif ; cette documentation n’établit pas la validation des jetons porteurs en production ou de deux appareils.

Le propriétaire vérifié des contrats est [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend), avec les [contrats de configuration v2](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) et les [interfaces v3](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md) additives. Les types portables appartenant au serveur et le transport client sont représentés localement dans `crates/cloud-contracts` et `crates/cloud-client`. La durabilité et l’authentification du bureau restent des responsabilités de l’hôte.

## Conditionner et préparer les opérations {#gate-and-stage-operations}

1. Découvrez les méthodes natives avec `rpc.discover` ; un ancien hôte peut conserver les vues héritées.
2. Laissez l’hôte vérifier `/api/v2/capabilities` de l’instance avant d’utiliser un point de terminaison conditionné par capacité. Ne codez jamais en dur une capacité annoncée par le serveur dans un simple indicateur du navigateur.
3. Utilisez l’origine provisionnée par l’opérateur. Les compilations de publication n’acceptent pas de remplacements arbitraires d’origine à l’exécution.
4. Prévisualisez migration et conflits, conservez les bases figées et écritures incertaines, et soumettez uniquement l’opération examinée.

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

Ces commandes examinent les schémas ; elles ne migrent pas de compte et n’enregistrent pas de valeurs de variables.

## Implémentation actuelle {#current-implementation}

V2 dispose d’un état/stockage/coordinateur natif, d’une migration v1 protégée, d’un examen des conflits, d’une reconstruction et de parcours de rapprochement des plugins installés. L’adoption est explicite et limitée au compte ; les anciennes installations non marquées conservent leur parcours historique. La validation complète de deux bureaux et certains adaptateurs produit restent incomplets. La configuration d’époque d’écriture à l’échelle du compte a des conséquences serveur irréversibles expliquées par son examen de migration.

V3 ajoute catalogue, équipes, intégrations, ressources, incrustations et paquets derrière des capacités. Les métadonnées de catalogue ne remplacent pas la vérification signée des plugins. L’importation utilise le journal du propriétaire ; examinez enabled à l’application/import de profil. L’application des paquets enregistre les étapes interrompues au lieu de les rejouer automatiquement. L’accès en lecture au relais Cloud ne doit pas être confondu avec l’autorité de mutation de collaboration directe.

## Résultat attendu et récupération {#expected-result-and-recovery}

Les changements de configuration conservent les frontières d’identité, de révision et d’examen. Automations Changes est uniquement local ; Variables avec examen utilise de vraies méthodes natives typées et ne synchronise pas les valeurs brutes. Overlay Changes n’est pas fusionné. En cas de conflit, comparez les enregistrements de base, locaux et Cloud récents ; en cas d’accusé inconnu, résolvez l’intention conservée. Une erreur HTTP autorisant une nouvelle tentative n’autorise jamais à répéter une mutation. La [collaboration](collaboration.md) couvre séparément l’autorité de l’hôte actif.

## Détails de fonctionnement {#operational-details}

`TIKTOOLS_CLOUD_ORIGIN` suit runtime → valeur compilée → défaut en debug, mais compilée → défaut en release, qui ignore les variables runtime. Vide, `off`, `none` désactivent seulement à la couche applicable. Valeur admise : origine HTTPS sans identifiants/chemin/query/fragment ; HTTP uniquement loopback explicite. Debug isolé : `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:8787 bun run start` ; release : `TIKTOOLS_CLOUD_ORIGIN=https://cloud.example bun scripts/task.ts rust build -p tiktools-desktop --release --locked`. Une identité d’une autre origine ne se restaure pas : vérifiez origine/compte avant authentification. Un défaut compilé ne prouve pas la disponibilité.

Suivez [migration et récupération par mode](../user/cloud/changes.md#operational-details). Coordonnez tous les appareils avant la politique d’époque irréversible, gardez comptes/résultats et relisez l’état après barrière ambiguë sans la répéter pour savoir si elle a réussi. La récupération fige entrées et révision examinées ; une revue périmée doit être refaite avant reprise, qui n’envoie pas d’écriture Cloud.
