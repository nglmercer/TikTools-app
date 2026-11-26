---
title: "Référence des commandes CLI"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "fr"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "c18e8235cae14c50e8091bd4611cf79fe175c67d6fc00ed5536c1cb9e3e3c554"
order: 75
---

# Référence des commandes CLI {#cli-command-reference}

## Objectif et prérequis {#goal-and-prerequisites}

Découvrez chaque forme de commande `tiktools` sans lire les sources Rust ni analyser la sortie d’aide humaine. Ayez un binaire `tiktools` correspondant compilé ; la référence est hors ligne et ne nécessite jamais de host actif. La source canonique est le registre de spécifications dans `crates/tiktools-cli/src/commands/spec.rs` ; chaque module de domaine possède les chaînes d’usage que son parseur rapporte dans les erreurs, donc cette référence ne peut pas diverger de ce que le parseur accepte.

## Inspecter l’aide hors ligne {#inspect-help-offline}

L’aide humaine regroupe les verbes par domaine ; le filtre affiche une section de domaine. La forme `--json` imprime le registre lisible par machine que tout agent devrait préférer : usage, résumé et marqueur destructif par verbe, plus les conventions globales de la CLI, versionné par le champ `schema` racine.

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

Les erreurs d’usage n’impriment que la forme pertinente avec une indication au lieu de la page complète, donc une faute de frappe ne fait jamais défiler le terminal. Le code de sortie 2 signifie toujours une erreur d’usage ; consultez le pied de page des conventions pour les règles de résolution du host, de sortie et de masquage des secrets.

## Référence des commandes générée {#generated-command-reference}

Générée depuis l’instantané d’aide CLI versionné dans `docs/contracts/cli-help.json` ; le binaire reste l’autorité. Les lignes d’usage sont la syntaxe du parseur et restent en anglais dans toutes les locales ; les résumés et conventions proviennent des catalogues de traduction de la CLI elle-même, donc cette référence ne peut pas diverger de ce que `tiktools help` imprime dans la même langue. Les verbes marqués destructifs modifient ou détruisent l’état ; les scripts devraient confirmer avant de les exécuter.

<!--@include: ../../../.generated/cli.fr.md-->

## Détails opérationnels {#operational-details}

L’instantané se régénère depuis un binaire compilé avec `--features local-web` pour que les domaines conditionnés par les features (cloud, publisher, web) soient complets. La vérification de provenance dans `docs check` compare les empreintes de chaque source Rust, donc tout changement de spécification de commande fait échouer la compilation jusqu’à la régénération du contrat. Les compilations normales de documentation n’exécutent jamais Cargo.

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

Exécutez la régénération après avoir modifié tout parseur ou spécification de commande, puis examinez le diff du fragment dans le cadre du changement. Les agents consommant `help --json` devraient traiter la version 1 du schéma comme stable de manière additive : de nouveaux verbes, domaines ou champs peuvent apparaître sans préavis ; les clés existantes conservent leur signification.
