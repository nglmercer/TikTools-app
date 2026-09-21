/**
 * Dedicated plugin-UI domain: contracts, normalization, bindings, actions,
 * and the legacy schema-v3 adapter. Import from here — never from
 * `src/automation` for UI concerns (the adapter is the only bridge, and it
 * is one-directional: automation types in, UI contract out).
 */

export {
  PLUGIN_UI_ACTION_TYPES,
  PLUGIN_UI_NODE_TYPES,
  PLUGIN_UI_VERSION,
  type CheckboxNode,
  type PluginUiAction,
  type PluginUiActionType,
  type PluginUiBinding,
  type PluginUiBindingScope,
  type PluginUiNode,
  type PluginUiNodeBase,
  type PluginUiNodeType,
  type PluginUiPage,
  type ButtonNode,
  type CardNode,
  type ConnectionNode,
  type FormNode,
  type ListNode,
  type RangeNode,
  type SelectNode,
  type SeparatorNode,
  type StackNode,
  type StatusNode,
  type TextNode,
  type TtsContribution,
  type TtsSettingsNode,
} from './contracts.ts';
export { parseBinding, settingsPathSegments } from './bindings.ts';
export { normalizeAction } from './actions.ts';
export {
  MAX_UI_CHILDREN,
  MAX_UI_DEPTH,
  MAX_UI_OPTIONS,
  collectOptionSources,
  normalizeNode,
  normalizePage,
} from './normalize.ts';
export { adaptLegacyPage, legacyTtsContributions, type AdaptedLegacyPage } from './legacy-v3-adapter.ts';
