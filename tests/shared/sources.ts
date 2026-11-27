/** Option sources that plugins declare for action fields. */
export const PLUGIN_SOURCE = {
  sonicboomVoices: 'plugin-action-options:sonicboom.server.speak:voice',
} as const;
export type PluginSource = (typeof PLUGIN_SOURCE)[keyof typeof PLUGIN_SOURCE];
