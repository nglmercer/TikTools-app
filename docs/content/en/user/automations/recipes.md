---
title: "Adapt practical automation recipes"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
translationRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
order: 30
---

# Adapt practical automation recipes {#adapt-practical-automation-recipes}

## Goal and prerequisites {#goal-and-prerequisites}

Turn a working [logging rule](first-rule.md) into narrowly matched reactions. You need an active profile and observed event fields. Begin with `core.log` actions; replace them with another effect only after confirming matching and frequency.

## Match a chat command {#match-a-chat-command}

Create a `tiktok.chat` event with `event.data.comment` equals (`eq`) `!hello`. Log:

```text
Hello {{ event.user.uniqueId }}
```

Use a per-user cooldown of `30` seconds so one viewer cannot continually trigger it while another viewer still has an independent opportunity. Exact equality is useful for a command; contains matching also accepts messages with surrounding text. Choose deliberately.

## Record a qualifying gift {#record-a-qualifying-gift}

Create a `tiktok.gift` event with `event.data.diamondCount` greater than or equal (`gte`) `10`. Log:

```text
Gift {{ event.data.giftName }} from {{ event.user.uniqueId }}
```

Inspect actual gift details first. A gift's diamond value, repeat count and completion flag are different fields. If you require a completion-only reaction, check that the incoming payload supplies `event.data.repeatEnd`, then use the boolean true condition. Do not infer one execution per completed combo from a name filter alone.

## Bound a shared reaction {#bound-a-shared-reaction}

Create a `tiktok.follow` event with a global cooldown of `10` seconds. Link a short log action. If two viewers follow during that interval, the global cooldown can suppress the later reaction. Use per-user scope instead when each viewer needs a separate allowance.

For multiple actions, order the list and keep all-actions mode when each step is required. Random mode chooses an action; it does not execute every step in random order. Add `core.delay` only when later steps should wait.

## Expected result and status {#expected-result-and-status}

These triggers, field filters and cooldown controls are available in the local engine. A run should correspond to an accepted matching event, subject to cooldown and enabled state. Provider-specific acceptance still needs your actual LIVE check.

## Recovery {#recovery}

If a recipe fails, return to a single log action, inspect the payload and temporarily simplify conditions. Do not disable limits on a busy stream while investigating. Audio, points, HTTP and plugin tests can perform real effects. Check [audio setup](../plugins/audio.md), [variables](variables.md) and [run diagnostics](../preferences/troubleshooting.md) before adding those dependencies.
