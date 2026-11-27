/**
 * The demo document's wire contract: URL parameters, frame messages, `<html>` dataset keys, limits
 * and timings. Dependency-free on purpose so the docs site, Playwright specs and scripts can import it.
 */
export const demoParam = {
  scenario: 'scenario', locale: 'locale', instance: 'instance', route: 'route', theme: 'theme',
  embed: 'embed', tour: 'tour', events: 'events', framed: 'framed', preview: 'preview', showcase: 'showcase', record: 'record',
} as const;
export type DemoParam = typeof demoParam[keyof typeof demoParam];
/** Boolean URL flags are exactly `1`; anything else is rejected by the parser. */
export const FLAG_ON = '1';
/** Parameters that only make sense inside the documentation frame, dropped for the "open full demo" link. */
export const frameOnlyParams: readonly DemoParam[] = [demoParam.embed, demoParam.showcase, demoParam.instance, demoParam.events, demoParam.framed, demoParam.theme];

export const frameMessageType = { state: 'tiktools.demo.state', command: 'tiktools.demo.command' } as const;

/** `document.documentElement.dataset` keys written by the demo. */
export const demoRootData = { panel: 'demoPanel', frame: 'demoFrame', showcase: 'demoShowcase', showcaseEntering: 'showcaseEntering', showcaseDone: 'showcaseDone' } as const;
/** Values of `data-demo-showcase`. */
export const showcaseMode = { live: 'true', record: 'record' } as const;

export const demoLimits = {
  /** Simulated events a visitor can play before the demo asks for a reset. */
  maxTicks: 300,
  maxInitialEvents: 12,
  maxShowcaseSteps: 12,
  /** Hard cap for any value that is echoed back in a state message. */
  maxMessageEvents: 300,
} as const;

export const demoTiming = {
  /** One simulated event advances the demo clock by this much. */
  tickMs: 1000,
  autoplayMs: 2000,
  metricsMs: 500,
} as const;

export const DEMO_STORAGE_PREFIX = 'tiktools.demo.';

/** Build a query string from known parameter names; empty values are skipped. */
export function demoSearchParams(values: Partial<Record<DemoParam, string | number | boolean | undefined>>): URLSearchParams {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(values)) {
    if (value === undefined || value === false || value === '') continue;
    params.set(key, value === true ? FLAG_ON : String(value));
  }
  return params;
}
