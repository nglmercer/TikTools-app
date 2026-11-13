---
title: "Reuse and review runtime variables"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
translationRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
order: 40
---

# Reuse and review runtime variables {#reuse-and-review-runtime-variables}

## Goal and prerequisites {#goal-and-prerequisites}

Change a shared value once and reuse it in several actions. You need **Automations → Variables** and a host that supports the relevant variable operations. Typed editing and reviewed Cloud operations appear according to host capabilities; local saving does not require uploading.

## Save a local value {#save-a-local-value}

1. Add a variable named `showLabel`. Keys start with a letter or underscore and may contain letters, digits, dots, underscores and hyphens, up to `64` characters. Avoid duplicate keys.
2. Where typed editing is available, choose text and enter `Evening show`. Other supported kinds are number, boolean, list and secret. Lists use a JSON array of strings, such as `["red", "blue"]`.
3. Select **Save**. A revision conflict means the saved collection changed; reload and review your intended edit against it.
4. Use the saved value in a templated action field:

```text
{{ globals.showLabel }}: {{ event.user.uniqueId }}
```

5. Test a log-only action to inspect the rendered text. Change `showLabel`, save again and run a new test. Existing action definitions can reuse the updated value without re-importing a template.

Template `params` are substituted when applying a template; `globals` are read at execution. A missing runtime key renders as empty text, so verify spelling before using it in a destination or identifier. Use the field's variable suggestions for event paths appropriate to its trigger.

## Review Cloud changes separately {#review-cloud-changes-separately}

The Variables **Changes** control has native reviewed send, fetch, apply/adopt and withdraw operations when supported. Save local edits first, select a comparison, inspect its type and value, and choose the intended direction explicitly. Fetching a Cloud candidate is separate from applying it locally. Withdrawing a shared value is separate from deleting a local row.

Secret or blocked values are masked in comparisons and cannot be sent through this reviewed path. Never turn a secret into ordinary text merely to bypass that block.

## Expected result, status and recovery {#expected-result-status-and-recovery}

Local variables and capability-gated reviewed sharing are available; saving locally never uploads automatically. Invalid keys, invalid typed values and stale revisions block saving. Correct the highlighted field or reload before retrying. For account failures see [Cloud account](../cloud/account.md); for comparison behavior see [Changes](../cloud/changes.md).
