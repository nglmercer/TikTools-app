/**
 * Injectable clock so controllers stay deterministic under test. Production
 * uses the real timers; tests inject a fake.
 */

export interface WidgetClock {
  now: () => number;
  setTimeout: (callback: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
}

export const realClock: WidgetClock = {
  now: () => Date.now(),
  setTimeout: (callback, ms) => setTimeout(callback, ms),
  clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
};
