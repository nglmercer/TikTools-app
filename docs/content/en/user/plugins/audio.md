---
title: "Set up sounds and speech"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b6be887705d13dfe120f3e07dc4006ec0efa1472b6dbe0ccfc1ebfc1e83f3672"
translationRevision: "b6be887705d13dfe120f3e07dc4006ec0efa1472b6dbe0ccfc1ebfc1e83f3672"
order: 20
---

# Set up sounds and speech {#set-up-sounds-and-speech}

## Goal and prerequisites {#goal-and-prerequisites}

Play a predictable sound before connecting audio to busy LIVE activity. You need a working output device and an existing supported audio file. Local **Play a sound** (`audio.play`) is a built-in action; speech providers such as an installed SonicBoom integration require their own plugin/service configuration.

## Test a local sound {#test-a-local-sound}

1. In **Automations → Actions**, create **Play a sound** (`audio.play`). Select an existing file with the media picker.
2. Set volume to `0.2` for a first check. The supported range is `0` to `1`; zero is silent.
3. Choose the overlap policy: `allow` permits concurrent playback, `restart` restarts the sound, and `drop` ignores a new instance while it is already playing.
4. Save the action and read the configuration before selecting **Test**. This test can play audible sound.
5. Listen through the intended device. After successful local playback, link the action to an event with a conservative [filter and cooldown](../automations/recipes.md).

The action retains a path-backed media reference. TikTools does not copy the audio file into its database. Keep the file at that path, and include it separately when moving or backing up your setup.

## Configure optional speech {#configure-optional-speech}

Install the trusted speech plugin through [Plugins](index.md). In its connection/settings controls, verify the service address, authentication and selected provider, language and voice/profile. Load a voice catalog before testing a short non-sensitive sentence. A missing voice should be investigated as a catalog, ownership or provider mismatch rather than replaced with an invented identifier.

For SonicBoom, synthesis, playback admission and audible completion are different outcomes. A queued or started response does not prove a person heard the sentence. Cloned speech can be slower than chat arrival, so inspect queue/saturation diagnostics and keep event admission bounded.

## Expected result, status and recovery {#expected-result-status-and-recovery}

Local audio playback is available when the host advertises audio support. Speech remains dependent on the installed plugin and reachable service; provider/device/platform acceptance is not guaranteed here. If silent, check file existence, volume, output device, pause state and the run error. If speech fails, check the service and voice catalog before retrying. Avoid repeated effect tests during a broadcast, and use [diagnostics](../preferences/troubleshooting.md) without sharing tokens or spoken private text.
