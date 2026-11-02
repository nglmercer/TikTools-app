import { defineStore } from 'pinia';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { createMediaPicker } from '@/shared/api/media-picker.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { usePlugins } from './plugins-state.ts';

export const usePluginsStore = defineStore('plugins', () => {
  const client = useControlClient();
  const automation = useAutomationStore();
  const media = createMediaPicker(client);
  const plugins = usePlugins(client, {
    translate: (key) => i18n.global.t(key),
    refreshBehavior: () => automation.refresh(),
    reportError: (message) => {
      automation.behaviorError = message;
    },
  });

  return { ...plugins, openMediaPicker: media.openMediaPicker };
});
