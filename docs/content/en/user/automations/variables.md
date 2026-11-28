---
title: "Reuse and review runtime variables"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1c64b76271cba5b5bb23ef1f671be2bd5bf6c286afb710d2bf85ccf8cecc66b0"
translationRevision: "1c64b76271cba5b5bb23ef1f671be2bd5bf6c286afb710d2bf85ccf8cecc66b0"
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

Conditions can also publish per-match values. When a filter declares a capture alias, the value that satisfied it is available while the event's actions render as <code v-pre>{{ match.&lt;alias&gt; }}</code> (or <code v-pre>{{ match.&lt;position&gt; }}</code> when no alias is set). For example, a `starts with` condition on the comment with alias `prefix` lets a log action print <code v-pre>{{ match.prefix }}</code>. Captures are bounded plain text, are only produced by the filters and alternatives that actually matched, and do not change cooldown keys.

Press **+** beside a templated field to open **Insert variable**: the exact path appears on the right and choosing a row appends <code v-pre>{{ path }}</code> to the field. This event's capture paths are listed there too, but they resolve only when their condition actually matched. Give a condition an alias in the Conditions section (the **Capture as** button in the footer reveals the alias fields) to write <code v-pre>{{ match.&lt;alias&gt; }}</code>; otherwise use the positional <code v-pre>{{ match.&lt;position&gt; }}</code>.

## Review Cloud changes separately {#review-cloud-changes-separately}

The Variables **Changes** control has native reviewed send, fetch, apply/adopt and withdraw operations when supported. Save local edits first, select a comparison, inspect its type and value, and choose the intended direction explicitly. Fetching a Cloud candidate is separate from applying it locally. Withdrawing a shared value is separate from deleting a local row.

Secret or blocked values are masked in comparisons and cannot be sent through this reviewed path. Never turn a secret into ordinary text merely to bypass that block.

## Expected result, status and recovery {#expected-result-status-and-recovery}

Local variables and capability-gated reviewed sharing are available; saving locally never uploads automatically. Invalid keys, invalid typed values and stale revisions block saving. Correct the highlighted field or reload before retrying. For account failures see [Cloud account](../cloud/account.md); for comparison behavior see [Changes](../cloud/changes.md).

Try it interactively before touching your real setup:

<DemoFrame scenario="automation" locale="en" panel="automation" title="Try the automations workspace" />

## Command line operations {#command-line-operations}

Every operation in this guide is also available from the CLI — offline, scriptable and agent-friendly. Usage lines are parser syntax and stay in English in every locale.

<!--@include: ../../../../.generated/cli-domain-globals.en.md-->

See the [full command reference](../../developer/cli.md) for conventions, JSON output and destructive-operation markers.

<TourFrame id="automationVariables" locale="en" title="Guided tour: Automations: variables" />
