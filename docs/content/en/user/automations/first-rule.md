---
title: "Create your first logging rule"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# Create your first logging rule {#create-your-first-logging-rule}

## Goal and prerequisites {#goal-and-prerequisites}

Verify the event-to-action path with one local log line. You need a running desktop host and access to **Automations**. No Cloud account, audio service or external endpoint is needed. Use fictional or non-sensitive chat text: logging records the rendered message.

## Create the action {#create-the-action}

1. Open **Automations → Actions** in the profile you want to use.
2. Create an action of type **Write to the log** (`core.log`) named `Chat check`.
3. Set its Message field to the following template, then save and enable this action:

```text
Chat check: {{ event.user.uniqueId }} said {{ event.data.comment }}
```

These are the current normalized chat paths. `event.data.comment` is the message; `event.user.uniqueId` is the viewer handle. A display name uses a different field.

## Create the event {#create-the-event}

1. Open **Events**, create `Chat check event` and select `tiktok.chat` as its trigger.
2. Link only `Chat check`. Keep the normal all-actions mode.
3. Leave filters empty for the first sample test. Set a global cooldown of `10` seconds to bound real traffic later.
4. Save and review any change preview. With only the logging action linked, use **Test** and inspect the returned run and log text. The host can supply a sample; a sample result does not establish real TikTok connectivity.
5. Add a condition on `event.data.comment`, operator equals (`eq`), value `!hello`. Save again and explicitly enable the event.
6. Connect to a [LIVE](../live/connections.md) you are authorized to test. Observe a new `!hello` chat in the [feed](../live/feed.md), then check automation history after the cooldown allows it.

## Expected result and status {#expected-result-and-status}

The local logging action and chat trigger are available. The sample should produce a readable log result; a matching real chat should produce a run with the actual handle and message. Other chat text should fail the exact-match condition.

## Recovery {#recovery}

If a sample stops matching after adding the filter, its supplied comment may differ from `!hello`. Check the real event fields, active profile and enabled states instead of repeatedly firing. Tests can run effects; keeping this rule log-only makes its effect predictable. Disable the event when finished, or continue with [recipes](recipes.md).

<DemoFrame scenario="automation" locale="en" tour="automation" title="Demo: automation" />
