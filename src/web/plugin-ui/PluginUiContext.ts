/**
 * Typed context consumed by the generic declarative plugin UI renderer.
 *
 * The renderer receives ONE stable context instead of 15–25 forwarded
 * props through App.vue and PluginPageView. Domain state (TTS, …) is NOT
 * part of this contract: domain panels register as custom node renderers
 * (`customNodes`) bound by the host at the composition root.
 */

import type { Component } from 'vue';

import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import type { JsonObject } from '../../automation/types.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';
import type { Locale } from '../i18n.ts';
import type { PluginSettingsState } from '../types.ts';

export interface PluginUiSettingsApi {
  state?: PluginSettingsState;
  get(): void;
  save(values: PluginSettingValues): void;
}

export interface PluginUiOptionsApi {
  values: Record<string, ActionOptionItem[]>;
  errors: Record<string, string>;
  selected: Record<string, string>;
  get(source: string, refresh?: boolean): void;
}

export interface PluginUiActionsApi {
  execute(actionType: string, config: Record<string, string | number | boolean>): void;
}

export interface PluginUiConnectionApi {
  state?: PluginConnectionState;
  test(): void;
}

export interface PluginUiMediaApi {
  pick?: OpenMediaPicker;
}

export interface PluginUiProvisioningApi {
  supported: boolean;
  state?: { working: boolean; ok: boolean; message: string };
  provision(username: string, password: string): void;
}

/** Local (ephemeral, never persisted) bindings backing `local.*` nodes. */
export interface PluginUiLocalState {
  get(name: string): string | number | boolean | undefined;
  set(name: string, value: string | number | boolean): void;
}

/**
 * Props every node renderer (built-in or custom) receives. Custom renderers
 * are host components injected by the composition root — never resolved
 * from manifest data.
 */
export interface PluginUiNodeProps {
  context: PluginUiContext;
  /** Path key identifying this node instance (`s0/s2/…`), for drafts. */
  nodeKey: string;
}

export interface PluginUiContext {
  locale: Locale;
  pluginId: string;
  pluginName: string;
  settings: PluginUiSettingsApi;
  options: PluginUiOptionsApi;
  actions: PluginUiActionsApi;
  connection: PluginUiConnectionApi;
  media: PluginUiMediaApi;
  provisioning: PluginUiProvisioningApi;
  local: PluginUiLocalState;
  /** Host-injected renderers for domain node types (`tts-settings`, …). */
  customNodes: Record<string, Component>;
  /** Per-node form drafts, keyed by `nodeKey`. */
  formDrafts: {
    get(key: string): JsonObject | undefined;
    set(key: string, value: JsonObject): void;
    keys(): string[];
    clear(): void;
  };
}

export function toSettingValues(value: JsonObject): PluginSettingValues {
  const clean: PluginSettingValues = {};
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string' || typeof entry === 'number' || typeof entry === 'boolean') {
      clean[key] = entry;
    }
  }
  return clean;
}
