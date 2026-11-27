/** `data-testid` values of the demo controls. Components bind these; specs query them. */
export const demoTestId = {
  more: 'demo-more',
  openControls: 'demo-open-controls',
  closeControls: 'demo-close-controls',
  controlsPanel: 'demo-controls-panel',
  play: 'demo-play',
  step: 'demo-step',
  reset: 'demo-reset',
  guide: 'demo-guide',
  closeGuide: 'demo-close-guide',
  playbackStatus: 'demo-playback-status',
  guideStatus: 'demo-guide-status',
  eventCount: 'demo-event-count',
  metrics: 'demo-metrics',
} as const;

/** Controls of the documentation frame that wraps the demo. */
export const frameTestId = { play: 'frame-play', step: 'frame-step', reset: 'frame-reset', zoom: 'frame-zoom' } as const;

/** Landing page stage that hosts the guided showcase. */
export const stageTestId = {
  tourToggle: 'stage-tour-toggle',
  step: (id: string) => `stage-step-${id}`,
} as const;
