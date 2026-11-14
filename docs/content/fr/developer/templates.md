---
title: "Modèles de règles, profils et examen du propriétaire"
docId: "developer.templates"
translationKey: "developer.templates"
locale: fr
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "b9c7c0a908b5ecf5dc10fc4f936cd9c956267059dc49d4a415215fffff3d8426"
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

Les importations de bibliothèque examinées créent des enregistrements liés cohérents et conservent un journal du propriétaire. Le catalogue Cloud conserve des modèles ; import de profils et application exigent une revue des valeurs enabled sans supposer leur désactivation. Les fournisseurs d’actions manquants ou désactivés provoquent une erreur de prérequis plutôt que de remplacer silencieusement une action. Un ensemble de profil sélectionné contrôle le contexte de présentation correspondant ; choisir un profil ne confère pas d’autorité distante.

## Récupérer et vérifier {#recover-and-verify}

En cas de conflit de révision, préservez la proposition, rechargez l’instantané de bibliothèque et examinez la différence réelle. Résolvez un journal natif en attente avant de créer une autre intention d’importation. Ne transformez pas un échec de mise à jour en création nouvelle implicite. Exécutez `bun scripts/task.ts test web rule-templates` et `bun scripts/task.ts test web library-journal` pour une couverture ciblée de l’interface. Le [service natif de modèles](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) et les schémas RPC générés remplacent les anciennes références `src/web/views/behavior`.

## Détails de fonctionnement {#operational-details}

Enregistrez le premier JSON comme `docs-log.tiktemplate.json` ; le second est un `docs-pack.tikprofile.json` complet d’une entrée. Les deux désactivent explicitement actions/événements. Priorité : défaut du schéma → profil → entrée → valeurs CLI/dialogue. Un span complet conserve le type ; un span incorporé devient texte. Enabled absent vaut true : utilisez `--disabled` et examinez la comparaison UI.

Limites actuelles : 8 actions et 12 filtres/modèle, 32 modèles/profil ; texte UI 64 KiB/modèle et 256 KiB/profil ; expansion profondeur 64, 65 536 nœuds, 256 KiB/enregistrement et 8 MiB total. Réduisez le contenu excessif sans contourner la validation. Les budgets bibliothèque, source et expansion sont distincts. Suivez [la récupération du journal](automations.md#operational-details) avant un nouvel import incertain.

```json
{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false,"filters":[],"cooldownMs":1000,"cooldownScope":"global","runMode":"all"}}
```

```json
{"profileVersion":1,"id":"docs-pack","name":"Docs pack","description":"","params":{"message":"profile"},"templates":[{"params":{"message":"entry"},"template":{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false}}}]}
```

```sh
tiktools template import docs-log.tiktemplate.json --dry-run
tiktools template import docs-log.tiktemplate.json --disabled
tiktools template profile-import docs-pack.tikprofile.json --disabled
```
