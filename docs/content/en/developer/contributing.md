---
title: "Contributing focused changes"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
translationRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
order: 230
---

# Contributing focused changes {#contributing-focused-changes}

## Goal and prerequisites {#goal-and-prerequisites}

Prepare a change that another contributor can review and verify. Read the repository's [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md), applicable `AGENTS.md` files and [setup](setup.md) before editing. Use Bun 1.4.4, Rust 1.95 or newer where needed, committed lockfiles and a branch based on the intended target.

## Define and implement the change {#define-and-implement-the-change}

1. Inspect `git status` and preserve work owned by others. Keep generated assets, credentials and local databases out of the patch.
2. Follow the current source owner: Rust core for service behavior, Control API modules for contracts, typed client for SDK coverage, and the [frontend layer map](frontend.md) for presentation.
3. For a contract change, update its canonical definition and regenerate derived output together. Do not hand-maintain another complete RPC or event schema table.
4. Keep plugin-specific implementations in their external plugin packages; generic host code should discover manifests rather than import a named plugin.
5. Explain the concrete trigger and resulting behavior, including incomplete or experimental paths.

## Validate proportionately {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

Native implementation changes also need the applicable Rust format/lint/tests and required CI gates. A documentation-only change should use docs checks and review its commands/source links rather than claim native runtime acceptance. The project task runner owns supported entry points; use the Cargo passthrough for specific native target flags.

## Expected result and status {#expected-result-and-status}

The review contains focused source changes, meaningful validation evidence and honest limits. Repository policy targets pull requests into `remake`; release tags are maintainer controlled. Contribution instructions are available, but branch protection and remote check configuration are administrator responsibilities rather than guarantees made by a Markdown page.

## Recover {#recover}

If validation fails because a dependency or platform library is missing, report and repair that environment rather than weakening a check. If generated files drift, regenerate from their owner and inspect the diff. Never publish session cookies, keys, credentials or unsanitized event captures. If source and a legacy design disagree, describe the discrepancy and use current implementation for public guidance. Do not overwrite another owner's pending files to make a broad cleanup look consistent.

## Operational details {#operational-details}

Review stamps have separate meanings: `sourceRevision` hashes the current English body, while `translationRevision` hashes this article’s localized body (frontmatter removed, outer whitespace trimmed). After reviewing source and all translations, run `bun docs/scripts/record-review.ts developer/example.md`. A later edit to either body invalidates the appropriate stamp. This detects drift, not human identity or semantic equivalence: compare commands, RPC/environment identifiers, payload keys, limits, activation/recovery steps and negations explicitly. Translate prose fully while preserving technical instructions.

For a split migration, record each original topic/heading and its concrete replacement destination in the migration map; include a heading fragment when the destination is a specific section. The checker validates existing files/fragments and exact archived bytes; editorial review must still establish thematic coverage. Never edit historical originals or their provenance to make a migration pass.
