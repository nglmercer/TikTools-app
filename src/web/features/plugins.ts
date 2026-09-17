import { ref } from 'vue';

import type { JsonObject } from '../../automation/types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import type {
  ActionOptionItem,
  HostMessage,
  PluginSettingValues,
} from '../../shared/messages.ts';
import {
  createInitialPluginInstallState,
  type PluginInstallState,
} from '../composables/plugin-install.ts';
import type { ControlClient } from '../platform/control-client.ts';
import { errorMessage } from '../platform/control-client.ts';
import type { PluginSettingsState } from '../types.ts';

export interface PluginSettingsResult {
  pluginId: string;
  schema: JsonObject;
  uiHints?: JsonObject;
  values: JsonObject;
}

export interface PluginOptionsResult {
  source: string;
  options: ActionOptionItem[];
  selected?: string | null;
}

export interface PluginConnectionResult {
  pluginId: string;
  ok: boolean;
  latencyMs: number;
  error?: string | null;
}

export interface PluginProvisionResult {
  pluginId: string;
  ok: boolean;
  error?: string | null;
}

export interface PluginActionOutcome {
  actionType: string;
  ok: boolean;
  summary: string;
  logs: string[];
  durationMs: number;
  error?: string | null;
}

export interface PluginsCallbacks {
  translate: (key: string) => string;
  refreshBehavior: () => Promise<void>;
  reportError: (message: string) => void;
  pickPluginPackage: (
    onSelected: (path: string | null, pickerError?: string) => void,
  ) => void;
}

/** Plugins: settings, options, health, provisioning, and installation. */
export function usePlugins(control: ControlClient, callbacks: PluginsCallbacks) {
  const pluginSettings = ref<Record<string, PluginSettingsState>>({});
  const actionOptions = ref<Record<string, ActionOptionItem[]>>({});
  const actionOptionErrors = ref<Record<string, string>>({});
  /** Server-reported selection per option source (option documents only). */
  const actionOptionSelected = ref<Record<string, string>>({});
  const pluginConnections = ref<Record<string, PluginConnectionState>>({});
  const pluginProvision = ref<Record<string, { working: boolean; ok: boolean; message: string }>>(
    {},
  );
  const pluginInstallState = ref<PluginInstallState>(createInitialPluginInstallState());
  const pluginProgress = ref<Extract<HostMessage, { type: 'plugin-progress' }> | null>(null);
  let pluginProgressTimer: ReturnType<typeof setTimeout> | undefined;

  const applySettings = (result: PluginSettingsResult): void => {
    pluginSettings.value = {
      ...pluginSettings.value,
      [result.pluginId]: {
        schema: result.schema,
        uiHints: result.uiHints,
        values: result.values,
      },
    };
  };

  const applyOptions = (result: PluginOptionsResult, error?: string): void => {
    actionOptions.value = { ...actionOptions.value, [result.source]: result.options };
    if (error) {
      actionOptionErrors.value = { ...actionOptionErrors.value, [result.source]: error };
    } else {
      const rest = { ...actionOptionErrors.value };
      delete rest[result.source];
      actionOptionErrors.value = rest;
    }
    // A fresh fetch without a selection clears the previous one so the
    // outputs selector never shows a value the server no longer reports.
    if (typeof result.selected === 'string') {
      actionOptionSelected.value = {
        ...actionOptionSelected.value,
        [result.source]: result.selected,
      };
    } else {
      const rest = { ...actionOptionSelected.value };
      delete rest[result.source];
      actionOptionSelected.value = rest;
    }
  };

  /** Runs one plugin action immediately (TTS speech, output switches). RPC
   * failures synthesize the same outcome shape the host would have sent. */
  const executeAction = async (
    actionType: string,
    config: PluginSettingValues,
    live: boolean,
  ): Promise<PluginActionOutcome> => {
    try {
      return await control.call<PluginActionOutcome>('plugins.action.execute', {
        actionType,
        config,
        live,
      });
    } catch (failure) {
      const message = errorMessage(failure);
      return { actionType, ok: false, summary: message, logs: [], durationMs: 0, error: message };
    }
  };

  control.onPush('plugin-settings', (message) => {
    if (message.type !== 'plugin-settings') return;
    applySettings({
      pluginId: message.id,
      schema: message.schema,
      uiHints: message.uiHints,
      values: message.values,
    });
  });
  control.onPush('plugin-progress', (message) => {
    if (message.type !== 'plugin-progress') return;
    pluginProgress.value = message;
    if (pluginProgressTimer) clearTimeout(pluginProgressTimer);
    if (message.state === 'ready' || message.state === 'failed') {
      pluginProgressTimer = setTimeout(
        () => {
          pluginProgress.value = null;
          pluginProgressTimer = undefined;
        },
        message.state === 'failed' ? 10_000 : 4_000,
      );
    }
  });

  const dismissPluginProgress = (): void => {
    pluginProgress.value = null;
    if (pluginProgressTimer) {
      clearTimeout(pluginProgressTimer);
      pluginProgressTimer = undefined;
    }
  };

  const handleGetPluginSettings = (id: string): void => {
    void control
      .call<PluginSettingsResult>('plugins.settings.get', { pluginId: id })
      .then(applySettings)
      .catch((failure: unknown) => {
        callbacks.reportError(errorMessage(failure));
      });
  };

  const handleSavePluginSettings = (id: string, values: PluginSettingValues): void => {
    void control
      .call<PluginSettingsResult>('plugins.settings.set', { pluginId: id, values })
      .then(applySettings)
      .catch((failure: unknown) => {
        callbacks.reportError(errorMessage(failure));
      });
  };

  const handleGetActionOptions = (source: string): void => {
    void control
      .call<PluginOptionsResult>('plugins.options', { source })
      .then((result) => applyOptions(result))
      .catch((failure: unknown) => {
        applyOptions({ source, options: [], selected: null }, errorMessage(failure));
      });
  };

  const handleTestPluginConnection = (id: string): void => {
    void control
      .call<PluginConnectionResult>('plugins.health', { pluginId: id })
      .then((result) => {
        pluginConnections.value = {
          ...pluginConnections.value,
          [result.pluginId]: {
            ok: result.ok,
            latencyMs: result.latencyMs,
            error: result.error ?? undefined,
            at: Date.now(),
          },
        };
      })
      .catch((failure: unknown) => {
        pluginConnections.value = {
          ...pluginConnections.value,
          [id]: { ok: false, latencyMs: 0, error: errorMessage(failure), at: Date.now() },
        };
      });
  };

  const handleProvisionPluginToken = (id: string, username: string, password: string): void => {
    pluginProvision.value = {
      ...pluginProvision.value,
      [id]: { working: true, ok: false, message: '' },
    };
    void control
      .call<PluginProvisionResult>('plugins.token.provision', {
        pluginId: id,
        username,
        password,
      })
      .then((result) => {
        pluginProvision.value = {
          ...pluginProvision.value,
          [result.pluginId]: {
            working: false,
            ok: result.ok,
            message: result.ok
              ? 'API token saved.'
              : (result.error ?? 'Provisioning failed.'),
          },
        };
        if (result.ok) {
          // The save path already refreshes the settings echo; re-probe so
          // the connection summary reflects the newly stored token.
          handleGetPluginSettings(result.pluginId);
          handleTestPluginConnection(result.pluginId);
        }
      })
      .catch((failure: unknown) => {
        pluginProvision.value = {
          ...pluginProvision.value,
          [id]: { working: false, ok: false, message: errorMessage(failure) },
        };
      });
  };

  const mutateInstallState = async (
    method: string,
    params: Record<string, unknown>,
  ): Promise<void> => {
    try {
      await control.call(method, params);
    } catch (failure) {
      callbacks.reportError(errorMessage(failure));
    }
    await callbacks.refreshBehavior();
  };

  const handleSetPluginInstalled = (id: string, installed: boolean): void => {
    void mutateInstallState('plugins.install.set', { pluginId: id, installed });
  };

  const handleUninstallPlugin = (id: string): void => {
    void mutateInstallState('plugins.uninstall', { pluginId: id });
  };

  const handleSetPluginEnabled = (id: string, enabled: boolean): void => {
    void mutateInstallState(enabled ? 'plugins.enable' : 'plugins.disable', { pluginId: id });
  };

  const sendInstallPackage = (path: string, replaceExisting: boolean): void => {
    pluginInstallState.value = {
      installing: true,
      error: '',
      success: '',
      pendingPath: path,
      needsReplace: false,
    };
    void control
      .call('plugins.install', { path, replaceExisting })
      .then(() => {
        pluginInstallState.value = {
          installing: false,
          error: '',
          success: callbacks.translate('pluginInstallSuccess'),
          pendingPath: '',
          needsReplace: false,
        };
        void callbacks.refreshBehavior();
      })
      .catch((failure: unknown) => {
        const message = errorMessage(failure);
        // Mirrors the host install-error classification: an existing
        // package id asks for replace confirmation instead of failing.
        if (/already installed/i.test(message)) {
          pluginInstallState.value = {
            ...pluginInstallState.value,
            installing: false,
            success: '',
            error: callbacks.translate('pluginReplaceConfirm'),
            needsReplace: true,
          };
          return;
        }
        pluginInstallState.value = {
          installing: false,
          error: message || callbacks.translate('pluginInstallFailed'),
          success: '',
          pendingPath: '',
          needsReplace: false,
        };
      });
  };

  const handleInstallPlugin = (): void => {
    if (pluginInstallState.value.installing) return;
    pluginInstallState.value = createInitialPluginInstallState();
    callbacks.pickPluginPackage((path, pickerError) => {
      if (pickerError) {
        pluginInstallState.value = {
          installing: false,
          error: pickerError,
          success: '',
          pendingPath: '',
          needsReplace: false,
        };
        return;
      }
      // Picker cancellation sends nothing.
      if (!path) return;
      sendInstallPackage(path, false);
    });
  };

  const handleConfirmPluginReplace = (): void => {
    const pending = pluginInstallState.value.pendingPath;
    if (!pending || pluginInstallState.value.installing) return;
    sendInstallPackage(pending, true);
  };

  const handleCancelPluginReplace = (): void => {
    pluginInstallState.value = createInitialPluginInstallState();
  };

  return {
    pluginSettings,
    actionOptions,
    actionOptionErrors,
    actionOptionSelected,
    pluginConnections,
    pluginProvision,
    pluginInstallState,
    pluginProgress,
    dismissPluginProgress,
    executeAction,
    handleGetPluginSettings,
    handleSavePluginSettings,
    handleGetActionOptions,
    handleTestPluginConnection,
    handleProvisionPluginToken,
    handleSetPluginInstalled,
    handleUninstallPlugin,
    handleSetPluginEnabled,
    handleInstallPlugin,
    handleConfirmPluginReplace,
    handleCancelPluginReplace,
  };
}
