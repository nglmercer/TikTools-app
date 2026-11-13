---
title: "Rule templates, profiles and owner review"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "11991768c81043f3784f28bca2800e7340aef0cbe9d74c71b5f6c2792c0f998d"
translationRevision: "11991768c81043f3784f28bca2800e7340aef0cbe9d74c71b5f6c2792c0f998d"
order: 160
---

# Rule templates, profiles and owner review

## Goal and prerequisites

Import reusable rules while preserving local ownership and recoverability. Use a private host, the required installed action providers and a valid `.tiktemplate.json` or `.tikprofile.json`. Read [automations](automations.md) before enabling an imported rule. Library import is configuration work, not evidence that its external integration is reachable.

## Prepare a document

A v1 template has `templateVersion`, a portable ID, parameter schema, actions and one linked event. A profile has `profileVersion`, shared parameters and inline template entries. The authoritative parsers are `crates/tiktools-core/src/services/templates/` and `src/web/entities/rule-template/lib/`; fixtures under `tests/fixtures/templates/` keep the two sides aligned.

Parameters merge from schema defaults to profile, entry and explicit dialog/CLI values. A whole <code v-pre>{{ params.* }}</code> span preserves the JSON value type; embedded spans become text. Event substitutions remain for execution time. Runtime globals resolve later and should not be baked into a shareable document as account secrets.

## Preview and import

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

These examples assume the file has been prepared and a matching host is running. In the UI, Rule templates and profile import stage a candidate for review. The native `automation.library.*` coordinator owns atomic library transitions and original-outcome recovery; its current schema is discoverable through the [Control API](control-api.md).

## Expected result and status

Reviewed library imports create coherent linked records and retain an owner journal. Cloud catalog template/profile imports stay disabled until intentional owner activation. Missing or disabled action providers cause a requirements error rather than silently replacing an action. One selected profile pack controls the corresponding presentation context; profile selection is not remote authority.

## Recover and verify

On revision conflict, preserve the candidate, reload the library snapshot and review the actual difference. Resolve a pending native journal before creating another import intent. Do not turn a failed update into an implicit new create. Run `bun scripts/task.ts test web rule-templates` and `bun scripts/task.ts test web library-journal` for focused frontend coverage. The [native template service](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) and generated RPC schemas supersede old `src/web/views/behavior` references.
