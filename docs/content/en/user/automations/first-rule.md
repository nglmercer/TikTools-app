---
title: "Create your first logging rule"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "13b4b7b3cb8eba95257ec9738934813266414f504eec8b43090b73e74ad50c10"
translationRevision: "13b4b7b3cb8eba95257ec9738934813266414f504eec8b43090b73e74ad50c10"
order: 20
---

# Create your first logging rule {#create-your-first-logging-rule}

## Goal and prerequisites {#goal-and-prerequisites}

Turn a chat event into one readable local log line. You need a running desktop host and access to **Automations**. No Cloud account, audio service or external endpoint is needed.

<DocCallout title="Start with a visible, local result" tone="tip">

Keep the first rule log-only so you can inspect its result. Use fictional or non-sensitive chat text: logging records the rendered message. The interactive example uses a prepared sample workspace that you can reset.

</DocCallout>

## Create the action {#create-the-action}

<DocSteps>

1. **Open the action editor.** Go to **Automations → Actions** in the profile you want to use.
2. **Name the action.** Create an action of type **Write to the log** (`core.log`) named `Chat check`.
3. **Set the message.** Enter the template below in **Message**, then save and enable the action.

</DocSteps>

```text
Chat check: {{ event.user.uniqueId }} said {{ event.data.comment }}
```

`event.data.comment` is the normalized chat message; `event.user.uniqueId` is the viewer handle. A display name uses a different field.

<DemoFrame scenario="automation" locale="en" panel="automation" title="Explore the automation editor" />

## Create the event {#create-the-event}

<DocSteps>

1. **Choose the trigger.** Open **Events**, create `Chat check event` and select `tiktok.chat`.
2. **Link the action.** Add only `Chat check` and keep the normal all-actions mode.
3. **Establish a baseline.** Leave filters empty for the first sample test. Set a global cooldown of `10` seconds to bound real traffic later.
4. **Test the result.** Save and review any change preview. With only the logging action linked, use **Test** and inspect the returned run and log text. A host-supplied sample does not establish real TikTok connectivity.
5. **Narrow the rule.** Add a condition on `event.data.comment`, operator equals (`eq`), value `!hello`. Save again and explicitly enable the event.
6. **Check a real chat.** Connect to a [LIVE](../live/connections.md) you are authorized to test. Observe a new `!hello` chat in the [feed](../live/feed.md), then check automation history after the cooldown allows it.

</DocSteps>

## Expected result and status {#expected-result-and-status}

The local logging action and chat trigger are available. A sample should produce a readable log result. A matching real chat should produce a run with the actual viewer handle and message; other chat text should fail the exact-match condition.

<DocCallout title="Check the complete path" tone="info">

Confirm the active profile, enabled action, enabled event, matching message and elapsed cooldown. Seeing a chat in the feed alone does not prove that an automation ran.

</DocCallout>

## Recovery {#recovery}

If a sample stops matching after adding the filter, its supplied comment may differ from `!hello`. Check the real event fields, active profile and enabled states instead of repeatedly firing. Tests can run effects; keeping this rule log-only makes its effect predictable. Disable the event when finished, or continue with [recipes](recipes.md).
