---
title: "Rule templates, profiles and owner review"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
order: 160
---

# Rule templates, profiles and owner review {#rule-templates-profiles-and-owner-review}

## Goal and prerequisites {#goal-and-prerequisites}

Import reusable rules while preserving local ownership and recoverability. Use a private host, the required installed action providers and a valid `.tiktemplate.json` or `.tikprofile.json`. Read [automations](automations.md) before enabling an imported rule. Library import is configuration work, not evidence that its external integration is reachable.

## Prepare a document {#prepare-a-document}

A v1 template has `templateVersion`, a portable ID, parameter schema, actions and one linked event. A profile has `profileVersion`, shared parameters and inline template entries. The authoritative parsers are `crates/tiktools-core/src/services/templates/` and `src/web/entities/rule-template/lib/`; fixtures under `tests/fixtures/templates/` keep the two sides aligned.

Parameters merge from schema defaults to profile, entry and explicit dialog/CLI values. A whole <code v-pre>{{ params.* }}</code> span preserves the JSON value type; embedded spans become text. Event substitutions remain for execution time. Runtime globals resolve later and should not be baked into a shareable document as account secrets.

## Preview and import {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

These examples assume the file has been prepared and a matching host is running. In the UI, Rule templates and profile import stage a candidate for review. The native `automation.library.*` coordinator owns atomic library transitions and original-outcome recovery; its current schema is discoverable through the [Control API](control-api.md).

## Expected result and status {#expected-result-and-status}

Reviewed library imports create coherent linked records and retain an owner journal. Cloud catalog template imports store documents; profile imports and later application require review of enabled values, not an assumption of disabled state. Missing or disabled action providers cause a requirements error rather than silently replacing an action. One selected profile pack controls the corresponding presentation context; profile selection is not remote authority.

## Recover and verify {#recover-and-verify}

On revision conflict, preserve the candidate, reload the library snapshot and review the actual difference. Resolve a pending native journal before creating another import intent. Do not turn a failed update into an implicit new create. Run `bun scripts/task.ts test web rule-templates` and `bun scripts/task.ts test web library-journal` for focused frontend coverage. The [native template service](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) and generated RPC schemas supersede old `src/web/views/behavior` references.

## Operational details {#operational-details}

Save the first JSON below as `docs-log.tiktemplate.json`; the second is a complete one-entry `docs-pack.tikprofile.json`. Both explicitly disable their actions/events. Parameter precedence is schema default → profile → entry → explicit CLI/dialog values; whole parameter spans retain types, embedded spans become strings. Omitted enabled values default true, so use `--disabled` for CLI imports and inspect the UI’s final comparison.

Current limits: 8 actions and 12 filters per template, 32 templates per profile; UI source text 64 KiB/template and 256 KiB/profile; expansion depth 64, 65,536 nodes, 256 KiB/record and 8 MiB aggregate. An expansion budget error requires reducing content, not bypassing validation. Library limits and source/expanded budgets are separate. Follow [journal recovery](automations.md#operational-details) before another import when a result is uncertain.

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
