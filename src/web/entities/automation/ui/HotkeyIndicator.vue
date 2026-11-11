<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../../shared/messages.ts';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import IconKeyboard from '@/shared/ui/icon/icons/IconKeyboard.vue';
import {
  formatHotkeyChord,
  hotkeyListenerState,
  hotkeyNeedsSeatAccess,
  summarizeHotkeyStatus,
} from '../lib/hotkey-status.ts';

const HOTKEYS_PLUGIN_ID = 'hotkeys';

const props = withDefaults(
  defineProps<{
    plugins: readonly PluginStatus[];
    status?: HotkeyStatusData | null;
    lastEvent?: { key: string; modifiers: string; at: number } | null;
    accessPending?: boolean;
  }>(),
  { status: null, lastEvent: null, accessPending: false },
);

const emit = defineEmits<{ requestAccess: [] }>();
const { t } = useI18n();

const plugin = computed(() => props.plugins.find((entry) => entry.descriptor.id === HOTKEYS_PLUGIN_ID));
const state = computed(() => (plugin.value?.enabled ? hotkeyListenerState(props.status) : 'disabled'));
const tone = computed(() => {
  if (state.value === 'active') return 'ok';
  if (state.value === 'permission' || state.value === 'failed') return 'error';
  return 'idle';
});
const details = computed(() => {
  const summary = summarizeHotkeyStatus(props.status);
  const last = props.lastEvent
    ? t('automations.hotkeys.last', { chord: formatHotkeyChord(props.lastEvent.key, props.lastEvent.modifiers) })
    : t('automations.hotkeys.noEvents');
  return [summary.headline, ...summary.lines.filter((line) => line !== summary.headline), last].join(' — ');
});
const canGrant = computed(() => tone.value === 'error' && hotkeyNeedsSeatAccess(props.status));
</script>

<template>
  <span
    v-if="plugin?.installed"
    v-tooltip="details"
    class="hotkey-indicator"
    :class="`hotkey-indicator--${tone}`"
    role="status"
    :aria-label="`${t('automations.hotkeys.title')}: ${t(`automations.hotkeys.states.${state}`)}`"
  >
    <!-- Icon and dot only; the title, state and last key are in the tooltip and the accessible name. -->
    <IconKeyboard />
    <span
      class="hotkey-indicator__dot"
      aria-hidden="true"
    />
    <BaseButton
      v-if="canGrant"
      size="sm"
      :disabled="accessPending"
      @click="emit('requestAccess')"
    >
      {{ t('automations.hotkeys.grant') }}
    </BaseButton>
  </span>
</template>

<style scoped>
.hotkey-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  color: var(--color-text-muted);
  white-space: nowrap;
}

.hotkey-indicator__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-subtle);
}

.hotkey-indicator--ok .hotkey-indicator__dot {
  background: var(--color-success);
}

.hotkey-indicator--error {
  color: var(--color-danger-hover);
}

.hotkey-indicator--error .hotkey-indicator__dot {
  background: var(--color-danger);
}
</style>
