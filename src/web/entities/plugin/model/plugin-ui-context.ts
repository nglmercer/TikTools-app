import type { Component } from 'vue';
import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { JsonObject } from '../../../../automation/types.ts';
import type { ActionOptionItem, OpenMediaPicker, PluginSettingValues } from '../../../../shared/messages.ts';
import type { PluginCredentialState, TokenSetupOptions } from '../../../../shared/token-import.ts';
import type { PluginSettingsState } from '@/types.ts';

export interface PluginUiContext {
  pluginId: string;
  pluginName: string;
  settings: {
    state?: PluginSettingsState;
    get(): void;
    save(values: PluginSettingValues): void;
  };
  options: {
    values: Readonly<Record<string, ActionOptionItem[]>>;
    errors: Readonly<Record<string, string>>;
    selected: Readonly<Record<string, string>>;
    get(source: string, refresh?: boolean, context?: JsonObject): void;
  };
  actions: {
    execute(actionType: string, config: PluginSettingValues): void;
  };
  connection: {
    state?: PluginConnectionState;
    test(): void;
  };
  media: {
    pick?: OpenMediaPicker;
  };
  provisioning: {
    supportsImport?: boolean;
    supportsDiscovery?: boolean;
    supported: boolean;
    state?: PluginCredentialState;
    importToken(options?: TokenSetupOptions): void;
    provision(username: string, password: string): void;
  };
  local: {
    get(name: string): string | number | boolean | undefined;
    set(name: string, value: string | number | boolean): void;
  };
  customNodes: Readonly<Record<string, Component>>;
  formDrafts: {
    get(key: string): JsonObject | undefined;
    set(key: string, value: JsonObject): void;
    keys(): string[];
    clear(): void;
  };
}
