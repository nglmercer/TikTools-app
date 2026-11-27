import { guideScopeSelector, guideTargetSelector } from '../../../src/web/features/guided-help/model/guide-dom.ts';

export { guideScopeSelector, guideTargetSelector };

/** CSS selectors of the documentation site and the demo document. */
export const selector = {
  stageFrame: '.product-stage iframe',
  panelFrame: '.demo-frame iframe',
  stageGuideLink: '.product-stage__guide',
  stageLesson: '.product-stage__lesson h3',
  stageBar: '.product-stage__bar',
  stageStep: '.product-stage__step',
  playground: '#playground',
  demoControls: '#demo-controls',
  appLayout: '.app-layout',
  liveFeed: '.live-feed',
  showcaseTip: '.showcase__tip',
  showcaseLayer: '.showcase',
} as const;

/** Selectors of the retired landing chrome; the landing must never render them again. */
export const retiredLandingSelectors = [
  '.product-stage__heading', '.product-stage__tabs', '.product-stage__flow', '.product-stage__bar > a', '.product-stage .demo-frame__bar',
] as const;

/** `<iframe>` of a focused documentation panel, e.g. `panelFrameFor('feed')`. */
export const panelFrameFor = (panel: string): string => `.demo-frame[data-panel="${panel}"] iframe`;
