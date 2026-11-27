/** CSS selectors the app e2e specs share, grouped by area. Page-side code cannot import these. */
export const SEL = {
  eventCard: '.event-card',
  eventCardGift: '.event-card--gift',
  eventCardContent: '.event-card__content',
  eventRow: '.event-row',
  eventDetails: '.event-details',
  feedStream: '.live-feed__stream',
  feedSearch: '.live-feed__search input',
  pluginFrame: '.plugin-frame',
  pluginsPage: '.plugins-page',
  pluginServerInput: '.plugin-servers__detail input',
  pluginServerPassword: '.plugin-servers__detail input[type="password"]',
  widgetHost: '.widget-host',
  followCard: '.follow-card',
  subscribeCard: '.subscribe-card',
  changeReviewCard: '.change-review__card',
  exploreRow: '.explore-list__row',
  baseJsonDiff: '.base-json-diff',
} as const;

/** Accessible names of plugin pages that several specs open. */
export const PLUGIN_TITLE = { tts: 'Text to Speech' } as const;
