import { storeToRefs } from 'pinia';
import type { Ref } from 'vue';
import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { pluginPickerOptions } from '@/entities/plugin/lib/plugin-install.ts';
import { usePluginsStore } from '@/entities/plugin/model/plugins-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { i18n } from '@/shared/i18n/i18n.ts';

export function usePluginControls() {
  const store = usePluginsStore();
  const automation = useAutomationStore();
  const packages = usePackagesStore();
  const refs = storeToRefs(store);
  const { behavior, behaviorError } = storeToRefs(automation);

  function installPackage(): void {
    store.openMediaPicker(pluginPickerOptions(i18n.global.t('plugins.messages.pickerTitle')), (selection, pickerError) => {
      if (pickerError) {
        automation.behaviorError = pickerError;
        return;
      }
      if (selection?.type === 'file') void packages.reviewSpec({ kind: 'file', path: selection.file.path });
    });
  }

  function installSpec(spec: string): void {
    void packages.reviewSpec({ kind: 'spec', spec });
  }

  return {
    behavior: behavior as unknown as Readonly<Ref<BehaviorSnapshot>>,
    behaviorError,
    settings: refs.pluginSettings,
    connections: refs.pluginConnections,
    provision: refs.pluginProvision,
    actionOptions: refs.actionOptions,
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
    installPackage,
    installSpec,
  };
}

export type PluginControls = ReturnType<typeof usePluginControls>;
