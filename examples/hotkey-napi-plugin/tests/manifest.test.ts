// Guards the manifest samples behind `test-event` and editor previews.
//
// `test_event` falls back to the declaring plugin's manifest sample when no
// live event of that type was seen yet, so every documented `event.data.*`
// field must be present or behaviors filtering on it can never pass the
// test even though live events carry it. Run with:
//
//   bun test examples/hotkey-napi-plugin/
//

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

interface ManifestEventType {
  type?: unknown;
  sample?: unknown;
}

function loadManifest(): { eventTypes: ManifestEventType[] } {
  const here = dirname(fileURLToPath(import.meta.url));
  const raw = readFileSync(join(here, "..", "plugin.json"), "utf8");
  return JSON.parse(raw) as { eventTypes: ManifestEventType[] };
}

function sampleOf(type: string): Record<string, unknown> {
  const entry = loadManifest().eventTypes.find((item) => item.type === type);
  if (!entry || typeof entry.sample !== "object" || entry.sample === null) {
    throw new Error(`manifest has no usable sample for ${type}`);
  }
  return entry.sample as Record<string, unknown>;
}

describe("hotkey manifest samples", () => {
  test("hotkey.pressed sample matches the live event shape", () => {
    // The guest emits exactly {key, modifiers, sequence, backend} per press
    // (see src/index.ts poll); the sample must carry the same keys.
    expect(Object.keys(sampleOf("hotkey.pressed")).sort()).toEqual(
      ["backend", "key", "modifiers", "sequence"],
    );
    expect(sampleOf("hotkey.pressed")).toMatchObject({
      key: "k",
      modifiers: "ctrl",
      sequence: "g k",
      backend: "rdev",
    });
  });

  test("hotkey.status sample matches the live status shape", () => {
    // statusEvent emits {platform, session, droppedEvents, pendingEvents,
    // backends}; a status filter on counters must match the sample too.
    expect(Object.keys(sampleOf("hotkey.status")).sort()).toEqual(
      ["backends", "droppedEvents", "pendingEvents", "platform", "session"],
    );
  });
});
