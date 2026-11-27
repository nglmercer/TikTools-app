/** Product limits that tests assert on; a unit test pins them to production. */
export const LIMIT = {
  /** Newest rows kept and rendered per event type in the live feed. */
  feedRenderLimit: 100,
} as const;
