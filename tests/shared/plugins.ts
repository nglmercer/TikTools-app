/** Plugin and registry ids used by fixtures. */
export const PLUGIN_ID = {
  sonicboomServer: 'sonicboom.server',
  sonicboomServerSpeak: 'sonicboom.server.speak',
  acmeTts: 'acme.tts',
  repoGame: 'repo.game',
} as const;
export type PluginId = (typeof PLUGIN_ID)[keyof typeof PLUGIN_ID];
