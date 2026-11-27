/** Values shared by the site specs: viewports, timeouts and the strings the demo is known to render. */
export const viewport = {
  desktop: { width: 1440, height: 960 },
  laptop: { width: 1280, height: 900 },
  mobile: { width: 390, height: 844 },
} as const;

export const timeout = {
  /** One tour step or a settled RPC. */
  step: 10_000,
  /** A showcase step reaching its next spotlight, including page load. */
  showcaseStep: 15_000,
  /** A complete recorded showcase run. */
  showcaseRun: 60_000,
  /** Poll interval while waiting for the showcase to progress. */
  showcasePoll: 250,
} as const;

/** The first scripted chat event of every demo feed. */
export const FIRST_CHAT_MESSAGE = 'Hello TikTools!';
