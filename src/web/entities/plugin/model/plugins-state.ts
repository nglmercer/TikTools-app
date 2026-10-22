import type { TokenSetupOptions, PluginCredentialState, TokenSetupCandidate } from '../../../../shared/token-import.ts';
import { ownControlSubscriptions } from '@/shared/api/control-owner.ts';
import { ref } from 'vue';

import type { JsonObject } from '../../../../automation/types.ts';
import type { PluginConnectionState, SonicBoomConnectionInfo } from '../../../../automation/plugins/declarative.ts';
import type {
  ActionOptionItem,
  HostMessage,
  PluginSettingValues,
} from '../../../../shared/messages.ts';
import {
  createInitialPluginInstallState,
  type PluginInstallState,
} from '@/entities/plugin/lib/plugin-install.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { PluginSettingsState } from '@/types.ts';

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
  serverInfo?: SonicBoomConnectionInfo;
  reachable?: boolean;
  gameplayReady?: boolean;
  details?: string;
  pluginId: string;
  ok: boolean;
  latencyMs: number;
  error?: string | null;
}

export interface PluginProvisionResult {
  candidates?: TokenSetupCandidate[];
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

export function usePlugins(control: ControlClient, callbacks: PluginsCallbacks) {
  const owner = ownControlSubscriptions(control);
  control = owner.client;
  let disposed = false;

  const pluginSettings = ref<Record<string, PluginSettingsState>>({});
  const actionOptions = ref<Record<string, ActionOptionItem[]>>({});
  const actionOptionErrors = ref<Record<string, string>>({});
  const actionOptionSelected = ref<Record<string, string>>({});
  const pluginConnections = ref<Record<string, PluginConnectionState>>({});
  const pluginProvision = ref<Record<string, PluginCredentialState>>(
    {},
  );
  const pluginInstallState = ref<PluginInstallState>(createInitialPluginInstallState());
  const pluginProgress = ref<Extract<HostMessage, { type: 'plugin-progress' }> | null>(null);
  let pluginProgressTimer: ReturnType<typeof setTimeout> | undefined;

  const applySettings = (result: PluginSettingsResult): void => {
    if (disposed) return;
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
    if (disposed) return;
    actionOptions.value = { ...actionOptions.value, [result.source]: result.options };
    if (error) {
      actionOptionErrors.value = { ...actionOptionErrors.value, [result.source]: error };
    } else {
      const rest = { ...actionOptionErrors.value };
      delete rest[result.source];
      actionOptionErrors.value = rest;
    }
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

  const executeAction = async (
    pluginId: string,
    actionType: string,
    config: PluginSettingValues,
    live: boolean,
  ): Promise<PluginActionOutcome> => {
    try {
      return await control.call<PluginActionOutcome>('plugins.action.execute', {
        pluginId,
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
  const applyProgress = (message: Extract<HostMessage, { type: 'plugin-progress' }>): void => {
    if (disposed) return;
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
  };
  control.onTopic<{
    pluginId: string;
    state: string;
    progress?: number | null;
    message: string;
  }>('plugin.progress', (data) => {
    const state = data.state as 'downloading' | 'loading' | 'ready' | 'failed';
    if (state !== 'downloading' && state !== 'loading' && state !== 'ready' && state !== 'failed') {
      return;
    }
    applyProgress({
      type: 'plugin-progress',
      pluginId: data.pluginId,
      state,
      progress: data.progress ?? undefined,
      message: data.message,
    });
  });
  for (const topic of [
    'plugin.installed',
    'plugin.uninstalled',
    'plugin.started',
    'plugin.stopped',
    'plugin.settings-changed',
  ]) {
    control.onTopic(topic, () => {
      void callbacks.refreshBehavior();
    });
  }

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
        if (disposed) return;
        callbacks.reportError(errorMessage(failure));
      });
  };

  const handleSavePluginSettings = (id: string, values: PluginSettingValues): void => {
    void control
      .call<PluginSettingsResult>('plugins.settings.set', { pluginId: id, values })
      .then(applySettings)
      .catch((failure: unknown) => {
        if (disposed) return;
        callbacks.reportError(errorMessage(failure));
      });
  };

  const handleGetActionOptions = (source: string, refresh?: boolean, pluginId?: string): void => {
    const params: Record<string, unknown> = { source };
    if (refresh) params.refresh = true;
    if (pluginId) params.pluginId = pluginId;
    void control
      .call<PluginOptionsResult>('plugins.options', params)
      .then((result) => applyOptions(result))
      .catch((failure: unknown) => {
        if (disposed) return;
        applyOptions({ source, options: [], selected: null }, errorMessage(failure));
      });
  };

  const handleTestPluginConnection = (id: string): void => {
    void control
      .call<PluginConnectionResult>('plugins.health', { pluginId: id })
      .then((result) => {
        if (disposed) return;
        pluginConnections.value = {
          ...pluginConnections.value,
          [result.pluginId]: {
            ok: result.ok,
            latencyMs: result.latencyMs,
            error: result.error ?? undefined,
            details: result.details,
            reachable: result.reachable,
            gameplayReady: result.gameplayReady,
            serverInfo: result.serverInfo,
            at: Date.now(),
          },
        };
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        pluginConnections.value = {
          ...pluginConnections.value,
          [id]: { ok: false, latencyMs: 0, error: errorMessage(failure), at: Date.now() },
        };
      });
  };

  const handleImportPluginToken = (id: string, options?: TokenSetupOptions): void => {
    if (pluginProvision.value[id]?.working) return;
    pluginProvision.value = { ...pluginProvision.value, [id]: { working: true, ok: false, message: '' } };
    void control.call<PluginProvisionResult>('plugins.token.import', { pluginId: id, ...options })
      .then((result) => {
        if (disposed) return;
        pluginProvision.value = {
          ...pluginProvision.value,
          [id]: { working: false, ok: result.ok, message: result.ok
            ? callbacks.translate('plugins.messages.tokenImported') : (result.error ?? ''),
            ...(result.candidates?.length ? { candidates: result.candidates } : {}) },
        };
        if (result.ok) {
          handleGetPluginSettings(id);
          handleTestPluginConnection(id);
        }
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        pluginProvision.value = { ...pluginProvision.value, [id]: { working: false, ok: false, message: errorMessage(failure) } };
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
        if (disposed) return;
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
          handleGetPluginSettings(result.pluginId);
          handleTestPluginConnection(result.pluginId);
        }
      })
      .catch((failure: unknown) => {
        if (disposed) return;
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
      if (disposed) return;
      callbacks.reportError(errorMessage(failure));
    }
    if (!disposed) await callbacks.refreshBehavior();
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
        if (disposed) return;
        pluginInstallState.value = {
          installing: false,
          error: '',
          success: callbacks.translate('plugins.messages.installSuccess'),
          pendingPath: '',
          needsReplace: false,
        };
        void callbacks.refreshBehavior();
      })
      .catch((failure: unknown) => {
        if (disposed) return;
        const message = errorMessage(failure);
        if (/already installed/i.test(message)) {
          pluginInstallState.value = {
            ...pluginInstallState.value,
            installing: false,
            success: '',
            error: callbacks.translate('plugins.messages.replaceConfirm'),
            needsReplace: true,
          };
          return;
        }
        pluginInstallState.value = {
          installing: false,
          error: message || callbacks.translate('plugins.messages.installFailed'),
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
      if (disposed) return;
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

  const dispose = (): void => {
    if (disposed) return;
    disposed = true;
    owner.dispose();
    if (pluginProgressTimer !== undefined) clearTimeout(pluginProgressTimer);
    pluginProgressTimer = undefined;
  };

  return {
    diagnostics: () => ({ timers: Number(pluginProgressTimer !== undefined), disposed }),
    dispose,
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
    handleImportPluginToken,
    handleSetPluginInstalled,
    handleUninstallPlugin,
    handleSetPluginEnabled,
    handleInstallPlugin,
    handleConfirmPluginReplace,
    handleCancelPluginReplace,
  };
}
