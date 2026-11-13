---
title: "Modèles de règles, profils et examen du propriétaire"
docId: "developer.templates"
translationKey: "developer.templates"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
translationRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
order: 160
---

# Modèles de règles, profils et examen du propriétaire {#rule-templates-profiles-and-owner-review}

## Objectif et prérequis {#goal-and-prerequisites}

Importer des règles réutilisables en préservant propriété locale et récupération. Utilisez un hôte privé, les fournisseurs d’actions installés requis et un fichier `.tiktemplate.json` ou `.tikprofile.json` valide. Lisez les [automatisations](automations.md) avant d’activer une règle importée. L’importation d’une bibliothèque est un travail de configuration, pas la preuve que son intégration externe est accessible.

## Préparer un document {#prepare-a-document}

Un modèle v1 comporte `templateVersion`, un identifiant portable, un schéma de paramètres, des actions et un événement lié. Un profil comporte `profileVersion`, des paramètres partagés et des entrées de modèles intégrées. Les analyseurs de référence se trouvent dans `crates/tiktools-core/src/services/templates/` et `src/web/entities/rule-template/lib/` ; les fixtures sous `tests/fixtures/templates/` maintiennent les deux côtés alignés.

Les paramètres fusionnent depuis les valeurs par défaut du schéma vers le profil, l’entrée et les valeurs explicites de boîte de dialogue/CLI. Une expression entière <code v-pre>{{ params.* }}</code> préserve le type de valeur JSON ; les expressions intégrées à du texte deviennent du texte. Les substitutions d’événement restent pour l’exécution. Les variables globales d’exécution sont résolues plus tard et ne doivent pas être figées comme secrets de compte dans un document partageable.

## Prévisualiser et importer {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

Ces exemples supposent le fichier préparé et un hôte correspondant actif. Dans l’interface, Rule templates et l’importation de profils préparent une proposition à examiner. Le coordinateur natif `automation.library.*` possède les transitions atomiques de bibliothèque et la récupération du résultat original ; son schéma actuel peut être découvert via l’[API de contrôle](control-api.md).

## Résultat attendu et état {#expected-result-and-status}

Les importations de bibliothèque examinées créent des enregistrements liés cohérents et conservent un journal du propriétaire. Les importations de modèles/profils du catalogue Cloud restent désactivées jusqu’à une activation intentionnelle du propriétaire. Les fournisseurs d’actions manquants ou désactivés provoquent une erreur de prérequis plutôt que de remplacer silencieusement une action. Un ensemble de profil sélectionné contrôle le contexte de présentation correspondant ; choisir un profil ne confère pas d’autorité distante.

## Récupérer et vérifier {#recover-and-verify}

En cas de conflit de révision, préservez la proposition, rechargez l’instantané de bibliothèque et examinez la différence réelle. Résolvez un journal natif en attente avant de créer une autre intention d’importation. Ne transformez pas un échec de mise à jour en création nouvelle implicite. Exécutez `bun scripts/task.ts test web rule-templates` et `bun scripts/task.ts test web library-journal` pour une couverture ciblée de l’interface. Le [service natif de modèles](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) et les schémas RPC générés remplacent les anciennes références `src/web/views/behavior`.
