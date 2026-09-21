/**
 * Deterministic clock for controller tests. Production code uses realClock.
 */

import type { WidgetClock } from './clock.ts';

export interface FakeClock {
  clock: WidgetClock;
  now: () => number;
  advance: (ms: number) => void;
  pendingTimers: () => number;
}

export function createFakeClock(start = 1_000_000): FakeClock {
  let now = start;
  let sequence = 0;
  const timers = new Map<number, { at: number; run: () => void }>();

  const clock: WidgetClock = {
    now: () => now,
    setTimeout: (run, ms) => {
      sequence += 1;
      timers.set(sequence, { at: now + Math.max(0, ms), run });
      return sequence;
    },
    clearTimeout: (handle) => {
      timers.delete(handle as number);
    },
  };

  const advance = (ms: number): void => {
    const target = now + ms;
    for (;;) {
      let earliest: number | null = null;
      for (const [id, timer] of timers) {
        if (timer.at <= target && (earliest === null || timer.at < (timers.get(earliest)?.at ?? Infinity))) {
          earliest = id;
        }
      }
      if (earliest === null) break;
      const timer = timers.get(earliest);
      timers.delete(earliest);
      if (timer) {
        now = timer.at;
        timer.run();
      }
    }
    now = target;
  };

  return { clock, now: () => now, advance, pendingTimers: () => timers.size };
}
