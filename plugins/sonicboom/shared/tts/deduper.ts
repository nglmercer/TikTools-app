import { normalizeHandle } from './settings.ts';
import type { TtsDeduperOptions } from './types.ts';

/** Fingerprint for one speakable chat line, used for de-duplication. */
export function ttsFingerprint(handle: string, spokenText: string): string {
  return `${normalizeHandle(handle)}\n${spokenText.trim().toLowerCase()}`;
}

/** Drops repeated deliveries of the same handle+text within a short window.
 * Guards against transport replays and double event emission speaking twice.
 * Points deduction must happen only after `claim` returns true, exactly once
 * per claimed fingerprint, so a replay can never double-charge. */
export class TtsDeduper {
  private readonly windowMs: number;
  private readonly maxEntries: number;
  private readonly seen = new Map<string, number>();

  constructor(options: TtsDeduperOptions = {}) {
    this.windowMs = Math.max(0, options.windowMs ?? 1500);
    this.maxEntries = Math.max(1, options.maxEntries ?? 500);
  }

  claim(fingerprint: string, now: number = Date.now()): boolean {
    const last = this.seen.get(fingerprint);
    if (last !== undefined && now - last < this.windowMs) {
      return false;
    }
    this.seen.set(fingerprint, now);
    if (this.seen.size > this.maxEntries) {
      // Allocation-free O(n) minimum scan. Strict `<` keeps the earliest
      // inserted entry on timestamp ties, matching the old stable sort.
      let oldestKey: string | undefined;
      let oldestAt = Infinity;
      for (const [key, at] of this.seen) {
        if (at < oldestAt) {
          oldestAt = at;
          oldestKey = key;
        }
      }
      if (oldestKey !== undefined) this.seen.delete(oldestKey);
    }
    return true;
  }

  clear(): void {
    this.seen.clear();
  }
}
