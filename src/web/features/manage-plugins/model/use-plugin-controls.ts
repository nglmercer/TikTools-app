import { storeToRefs } from 'pinia';
import type { Ref } from 'vue';
import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';

export function usePluginControls() {
  const store = usePluginsStore();
  const automation = useAutomationStore();
  const refs = storeToRefs(store);
  const { behavior, behaviorError } = storeToRefs(automation);

  return {
    behavior: behavior as unknown as Readonly<Ref<BehaviorSnapshot>>,
    behaviorError,
    settings: refs.pluginSettings,
    connections: refs.pluginConnections,
    provision: refs.pluginProvision,
    actionOptions: refs.actionOptions,
    installState: refs.pluginInstallState,
    openMediaPicker: store.openMediaPicker,
    loadSettings: store.handleGetPluginSettings,
    saveSettings: store.handleSavePluginSettings,
    loadOptions: (source: string) => store.handleGetActionOptions(source),
    testConnection: store.handleTestPluginConnection,
    executeAction: store.executeAction,
    importToken: store.handleImportPluginToken,
    provisionToken: store.handleProvisionPluginToken,
    setInstalled: store.handleSetPluginInstalled,
    setEnabled: store.handleSetPluginEnabled,
    uninstall: store.handleUninstallPlugin,
    installPackage: store.handleInstallPlugin,
    confirmReplace: store.handleConfirmPluginReplace,
    cancelReplace: store.handleCancelPluginReplace,
  };
}

export type PluginControls = ReturnType<typeof usePluginControls>;
