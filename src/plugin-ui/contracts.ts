/**
 * Canonical declarative plugin UI contract (uiVersion 1).
 *
 * Plugins describe UI as data; the TikTools frontend renders trusted host
 * components. Manifests never inject HTML, JavaScript, components, or CSS
 * into the main document — the main WebView holds the privileged IPC
 * bridge, so arbitrary plugin script must never execute there.
 *
 * This module is the dedicated plugin-UI domain. It intentionally does NOT
 * depend on `src/automation`: plugin configuration UI is not automation.
 * The legacy schema-v3 transport (`BehaviorSnapshot.pluginPages`) is
 * converted here by `legacy-v3-adapter.ts`, keeping backward compatibility
 * while the renderer only speaks this contract.
 *
 * Mirror: `crates/tiktools-plugin-api/src/ui/` is the canonical Rust
 * contract (validation + JsonSchema derives). The TypeScript types below
 * mirror it; validation parity is covered by fixture tests on both sides.
 */

import type { Localized } from '../shared/localized.ts';
import type { JsonObject } from '../shared/json.ts';

/** Contract version carried by plugin manifests (`ui.uiVersion`). */
export const PLUGIN_UI_VERSION = 1;

/**
 * Generic node types. Deliberately closed: adding a domain (obs, discord,
 * …) must compose these primitives, never extend this union with a
 * domain-specific section kind.
 */
export type PluginUiNodeType =
  | 'stack'
  | 'card'
  | 'text'
  | 'form'
  | 'select'
  | 'range'
  | 'checkbox'
  | 'button'
  | 'list'
  | 'status'
  | 'separator'
  | 'connection'
  | 'tts-settings';

export const PLUGIN_UI_NODE_TYPES: readonly PluginUiNodeType[] = [
  'stack',
  'card',
  'text',
  'form',
  'select',
  'range',
  'checkbox',
  'button',
  'list',
  'status',
  'separator',
  'connection',
  'tts-settings',
];

/**
 * Allowlisted UI action types. Declarative buttons/events reference one of
 * these descriptors; there is no generic arbitrary-RPC invocation.
 */
export type PluginUiActionType =
  | 'save-settings'
  | 'plugin-action'
  | 'refresh-source'
  | 'test-connection'
  | 'open-media-picker';

export const PLUGIN_UI_ACTION_TYPES: readonly PluginUiActionType[] = [
  'save-settings',
  'plugin-action',
  'refresh-source',
  'test-connection',
  'open-media-picker',
];

/** Safe action descriptor attached to buttons and events. */
export type PluginUiAction =
  | { type: 'save-settings' }
  | { type: 'plugin-action'; actionType: string; config?: JsonObject }
  | { type: 'refresh-source'; source: string }
  | { type: 'test-connection' }
  | { type: 'open-media-picker'; accept?: string; target?: string };

/**
 * Limited binding scopes. No expressions, no eval, no function calls:
 * - `settings.<dotted-path>` — plugin settings values (read/write).
 * - `local.<name>` — page-local ephemeral state (never persisted).
 * - `source.<key>` — host-resolved option-source metadata (read-only).
 */
export type PluginUiBindingScope = 'settings' | 'local' | 'source';

export interface PluginUiBinding {
  raw: string;
  scope: PluginUiBindingScope;
  /** Dotted path within the scope (settings) or bare name (local/source). */
  path: string;
}

export interface PluginUiNodeBase {
  type: PluginUiNodeType;
  /** Optional stable key for per-node state (form drafts). */
  key?: string;
  label?: Localized;
  title?: Localized;
}

export interface StackNode extends PluginUiNodeBase {
  type: 'stack';
  children: PluginUiNode[];
}

export interface CardNode extends PluginUiNodeBase {
  type: 'card';
  children: PluginUiNode[];
}

export interface TextNode extends PluginUiNodeBase {
  type: 'text';
  /** Plain text; rendered as text, never markup. */
  text: Localized;
}

export interface FormNode extends PluginUiNodeBase {
  type: 'form';
  /** Defaults to the plugin's full settings schema. */
  schema?: JsonObject;
  /** Defaults to the plugin's settings UI hints. */
  uiHints?: JsonObject;
}

export interface SelectNode extends PluginUiNodeBase {
  type: 'select';
  bind: string;
  /** Static fallback options; `optionsFrom` wins when present. */
  options?: Array<{ value: string; label: Localized }>;
  optionsFrom?: string;
}

export interface RangeNode extends PluginUiNodeBase {
  type: 'range';
  bind: string;
  min?: number;
  max?: number;
  step?: number;
}

export interface CheckboxNode extends PluginUiNodeBase {
  type: 'checkbox';
  bind: string;
}

export interface ButtonNode extends PluginUiNodeBase {
  type: 'button';
  action: PluginUiAction;
  variant?: 'primary' | 'danger';
}

export interface ListNode extends PluginUiNodeBase {
  type: 'list';
  optionsFrom: string;
}

export interface StatusNode extends PluginUiNodeBase {
  type: 'status';
  /** Severity is host-derived from bound state; never plugin-styled HTML. */
  tone?: 'info' | 'ok' | 'error';
  text: Localized;
}

export interface SeparatorNode extends PluginUiNodeBase {
  type: 'separator';
}

export interface ConnectionNode extends PluginUiNodeBase {
  type: 'connection';
}

/**
 * Host-rendered TTS panel bound to a TTS contribution id. The generic
 * renderer never imports TTS UI: the host injects the concrete renderer
 * for this node type (see `PluginUiContext.customNodes`).
 */
export interface TtsSettingsNode extends PluginUiNodeBase {
  type: 'tts-settings';
  contribution: string;
}

export type PluginUiNode =
  | StackNode
  | CardNode
  | TextNode
  | FormNode
  | SelectNode
  | RangeNode
  | CheckboxNode
  | ButtonNode
  | ListNode
  | StatusNode
  | SeparatorNode
  | ConnectionNode
  | TtsSettingsNode;

/** One normalized plugin UI page: header plus a generic node body. */
export interface PluginUiPage {
  id: string;
  pluginId: string;
  title: Localized;
  icon?: string;
  body: PluginUiNode;
}

/**
 * Explicit TTS domain contribution. Auto-TTS, speech dispatch, and voice
 * policy key off contributions — never off walking UI sections. Schema-v3
 * `"kind": "tts"` sections generate one contribution plus a `tts-settings`
 * node during adaptation; future manifests may declare contributions
 * directly without any TTS UI section.
 */
export interface TtsContribution {
  /** Stable within the plugin (`main` for single-TTS plugins). */
  id: string;
  pluginId: string;
  /** Plugin action type executed for speech. */
  actionType: string;
  /** Normalized option source id feeding voice selectors. */
  voicesFrom: string;
  /** Optional normalized source id feeding the output selector. */
  outputsFrom?: string;
}
