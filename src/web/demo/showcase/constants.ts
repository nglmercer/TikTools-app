/**
 * Timing and motion of the showcase presentation, in milliseconds unless noted.
 * The selectors in showcase.css (`[data-on='true']`, `[data-still='true']`, `[data-visible='true']`)
 * mirror `layerData`; change both together.
 */
export const spotlightSettleMs = 560;
/** Keep following the layout this long after the spotlight starts moving. */
export const spotlightFollowMs = 400;
export const glideEasing = 'cubic-bezier(0.22, 1, 0.36, 1)';
/** Cursor travel: a base time plus time per pixel, capped. */
export const cursorTravel = { baseMs: 380, perPx: 0.9, maxMs: 1100 } as const;
export const clickPressMs = 260;
export const rippleMs = 520;
export const easeOut = 'ease-out';
export const clickSettleMs = 220;
export const cursorHideMs = 900;
/** How long a `data-guide` target is polled before its step is skipped. */
export const targetWaitMs = 4000;
/** How long the fade-in marker stays on the root after a view change without View Transitions. */
export const viewEnterMs = 420;
export const layerData = { on: 'on', still: 'still', visible: 'visible' } as const;
