/** Automation action type ids. */
export const ACTION_TYPE = {
  coreLog: 'core.log',
  coreFetch: 'core.fetch',
  deckScene: 'deck.scene',
  demoTts: 'demo.tts',
} as const;
export type ActionType = (typeof ACTION_TYPE)[keyof typeof ACTION_TYPE];
