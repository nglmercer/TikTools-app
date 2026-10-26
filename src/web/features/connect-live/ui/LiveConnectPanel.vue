<script setup lang="ts">
import { computed, markRaw, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { isConnectionBusy } from '@/entities/connection/lib/connection-status.ts';
import ConnectionIndicator from '@/entities/connection/ui/ConnectionIndicator.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import IconDice from '@/shared/ui/icon/icons/IconDice.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import BaseMenu from '@/shared/ui/menu/BaseMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import type { ConnectionStatus } from '@/types.ts';

const props = defineProps<{
  status: ConnectionStatus;
  creator: string;
  recents: readonly string[];
  error: string;
  reconnectAttempt?: number | null;
  reconnectDelayMs?: number | null;
  maxReconnectAttempts?: number;
  monitorEnabled?: boolean;
  monitorPhase?: string;
  monitorBusy?: boolean;
  monitorError?: string;
}>();

const handle = defineModel<string>('handle', { required: true });
const cookie = defineModel<string>('cookie', { required: true });
const useCookie = defineModel<boolean>('useCookie', { default: false });

const emit = defineEmits<{
  connect: [];
  disconnect: [];
  reconnect: [];
  'pick-random': [];
  'start-monitor': [];
  'stop-monitor': [];
  'select-recent': [handle: string];
}>();

const { t } = useI18n();
const recentsButton = useTemplateRef('recentsButton');
const recentsOpen = ref(false);

const live = computed(() => props.status === 'connected');
const busy = computed(() => isConnectionBusy(props.status));
const locked = computed(() => live.value || busy.value || props.monitorEnabled || props.monitorBusy);
const recentItems = computed<MenuEntry[]>(() =>
  props.recents.map((entry) => {
    const clean = entry.replace(/^@/, '');
    return { id: clean, label: `@${clean}`, icon: markRaw(IconUser) };
  }),
);

function submit(): void {
  if (!locked.value && handle.value.trim()) emit('connect');
}

function onRecent(action: MenuAction): void {
  emit('select-recent', action.id);
}
</script>

<template>
  <BasePanel :title="t('connections.live.title')">
    <template #actions>
      <ConnectionIndicator
        :status="status"
        :reconnect-attempt="reconnectAttempt"
        :reconnect-delay-ms="reconnectDelayMs"
        :max-reconnect-attempts="maxReconnectAttempts"
      />
    </template>
    <form
      class="live-connect"
      @submit.prevent="submit"
    >
      <BaseFormGrid>
        <BaseFormRow
          for="live-handle"
          :label="t('connections.live.creator')"
          :hint="live ? t('connections.live.lockedHint') : t('connections.live.handleHint')"
        >
          <span class="live-connect__handle">
            <BaseTextField
              id="live-handle"
              v-model="handle"
              class="live-connect__field"
              :placeholder="t('connections.live.handlePlaceholder')"
              :disabled="locked"
              autocomplete="off"
              spellcheck="false"
            >
              <template #prefix>
                @
              </template>
            </BaseTextField>
            <span ref="recentsButton">
              <BaseIconButton
                :label="t('connections.live.recents')"
                variant="default"
                :disabled="locked || !recents.length"
                @click="recentsOpen = !recentsOpen"
              >
                <IconHistory />
              </BaseIconButton>
            </span>
            <BaseIconButton
              :label="t('connections.live.pickRandom')"
              variant="default"
              :disabled="locked"
              @click="emit('pick-random')"
            >
              <IconDice />
            </BaseIconButton>
          </span>
        </BaseFormRow>
        <span />
        <BaseCheckbox
          v-model="useCookie"
          :disabled="locked"
        >
          {{ t('connections.live.useCookie') }}
        </BaseCheckbox>
        <BaseFormRow
          v-if="useCookie"
          for="live-cookie"
          :label="t('connections.live.cookie')"
          :hint="t('connections.live.cookieHint')"
        >
          <BaseTextField
            id="live-cookie"
            v-model="cookie"
            type="password"
            :placeholder="t('connections.live.cookiePlaceholder')"
            :disabled="locked"
            autocomplete="off"
          />
        </BaseFormRow>
      </BaseFormGrid>
      <p
        v-if="monitorEnabled"
        role="status"
      >
        {{ t('connections.monitor.status', { phase: t(`connections.monitor.phases.${monitorPhase || 'waiting'}`) }) }}
      </p>
      <p
        v-if="monitorError"
        role="alert"
        class="live-connect__error"
      >
        {{ monitorError }}
      </p>
      <p
        v-if="error"
        class="live-connect__error"
        role="alert"
      >
        {{ error }}
      </p>
      <p
        v-if="live && creator"
        class="live-connect__session"
      >
        {{ t('connections.live.connectedTo', { creator: `@${creator.replace(/^@/, '')}` }) }}
      </p>
    </form>
    <BaseMenu
      v-model:open="recentsOpen"
      :items="recentItems"
      :anchor="recentsButton"
      placement="bottom-end"
      @select="onRecent"
    />
    <template #footer>
      <span class="live-connect__spacer" />
      <BaseButton
        v-if="monitorEnabled"
        variant="danger"
        :disabled="monitorBusy"
        @click="emit('stop-monitor')"
      >
        {{ t('connections.monitor.stop') }}
      </BaseButton>
      <template v-else-if="live">
        <BaseButton @click="emit('reconnect')">
          {{ t('connection.reconnect') }}
        </BaseButton>
        <BaseButton
          variant="danger"
          @click="emit('disconnect')"
        >
          {{ t('connection.disconnect') }}
        </BaseButton>
      </template>
      <BaseButton
        v-else-if="busy"
        variant="danger"
        @click="emit('disconnect')"
      >
        {{ t('common.cancel') }}
      </BaseButton>
      <template v-else>
        <BaseButton
          :disabled="monitorBusy || !handle.trim()"
          @click="emit('start-monitor')"
        >
          {{ t('connections.monitor.start') }}
        </BaseButton>
        <BaseButton
          v-if="status === 'error'"
          @click="emit('reconnect')"
        >
          {{ t('connection.reconnect') }}
        </BaseButton>
        <BaseButton
          variant="primary"
          :disabled="!handle.trim()"
          @click="submit"
        >
          {{ t('connection.connect') }}
        </BaseButton>
      </template>
    </template>
  </BasePanel>
</template>

<style scoped>
.live-connect {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.live-connect__handle {
  display: flex;
  gap: var(--space-2);
}

.live-connect__field {
  flex: 1;
}

.live-connect__error {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.live-connect__session {
  margin: 0;
  color: var(--color-text-muted);
}

.live-connect__spacer {
  flex: 1;
}
</style>
