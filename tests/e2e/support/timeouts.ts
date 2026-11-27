/** Waits (ms) for expectations that depend on a frame, a lazy route or a fixture round trip. */
export const WAIT_MS = {
  short: 5_000,
  frame: 10_000,
  long: 20_000,
} as const;
