---
title: "CLI command reference"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
order: 75
---

# CLI command reference {#cli-command-reference}

## Goal and prerequisites {#goal-and-prerequisites}

Discover every `tiktools` command form without reading Rust sources or parsing human help output. Have a matching `tiktools` binary built; the command reference is offline and never needs a running host. The canonical source is the command spec registry in `crates/tiktools-cli/src/commands/spec.rs`; each domain module owns the usage strings its parser reports in errors, so this reference cannot drift from what the parser accepts.

## Inspect help offline {#inspect-help-offline}

Human help groups verbs by domain; filtering renders one domain section. The `--json` form prints the machine-readable registry every agent should prefer: usage, summary and a destructive marker per verb, plus the global CLI conventions, versioned by the top-level `schema` field.

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

Usage errors print only the relevant form with a hint instead of the full page, so a typo never scrolls the terminal. Exit code 2 always means a usage error; see the conventions footer for host resolution, output and secret-redaction rules.

## Generated command reference {#generated-command-reference}

Generated from the checked-in CLI help snapshot in `docs/contracts/cli-help.json`; the binary remains authoritative. Usage lines are parser syntax and stay in English in every locale; summaries and conventions come from the CLI's own translation catalogs, so this reference cannot diverge from what `tiktools help` prints in the same language. Verbs marked destructive mutate or destroy state; scripts should confirm before executing them.

<!--@include: ../../../.generated/cli.en.md-->

## Operational details {#operational-details}

The snapshot regenerates from a binary built with `--features local-web` so feature-gated domains (cloud, publisher, web) are complete. The provenance check in `docs check` compares hashes of every Rust source, so any command-spec change fails the build until the contract is regenerated. Normal docs builds never run Cargo.

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

Run the regeneration after changing any command parser or spec, then review the fragment diff as part of the change. Agents consuming `help --json` should treat schema version 1 as additive-stable: new verbs, domains or fields may appear without notice, existing keys keep their meaning.
