/** Waits (ms) for expectations that depend on a frame, a lazy route or a fixture round trip. */
export const WAIT_MS = {
  short: 5_000,
  frame: 10_000,
  long: 20_000,
} as const;

/** Deliberate real-time pauses: each one waits for something that has no observable signal. */
export const PAUSE_MS = {
  /** Disposed RPC chains need a browser turn before a heap snapshot. */
  heapSettle: 250,
  /** Gap between replay clicks so each alert starts its own transition. */
  replayGap: 150,
  /** Lets the last alert transition finish before the monitor is read. */
  transitionEnd: 700,
  /** Negative check: an event from another plugin must not reach the frame. */
  foreignEvent: 300,
  /** The sample must stay visible beyond the normal subscription alert duration. */
  pastSubscriptionAlert: 5_500,
} as const;

/** Whole-test budgets (ms) for the heap and lifecycle specs that repeat 100 cycles. */
export const TEST_BUDGET_MS = { lifecycle: 120_000 } as const;
