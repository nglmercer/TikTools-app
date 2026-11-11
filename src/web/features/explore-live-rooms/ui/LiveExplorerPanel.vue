<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { storeToRefs } from 'pinia';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useDiscoveryStore } from '@/entities/live-room/model/discovery-store.ts';
import { formatCount } from '@/shared/lib/intl.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconDice from '@/shared/ui/icon/icons/IconDice.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import BaseTable from '@/shared/ui/table/BaseTable.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import type { LiveChannel } from '@/types.ts';

const props = defineProps<{
  currentCreator: string;
}>();

const emit = defineEmits<{ connect: [room: LiveChannel] }>();

const { t, locale } = useI18n();
const store = useDiscoveryStore();
const { open, keyword, rooms, status, error, notice, nextOffset, randomCandidate } = storeToRefs(store);

const columns = computed(() => [
  { key: 'uniqueId', label: t('connections.explore.creator'), width: '34%' },
  { key: 'title', label: t('connections.explore.roomTitle') },
  { key: 'viewers', label: t('connections.explore.viewers'), width: '110px', align: 'end' as const },
  { key: 'connect', label: '', width: '104px', align: 'end' as const },
]);

const statusMessage = computed(() => {
  if (notice.value === 'stale') return t('connections.explore.stale');
  if (notice.value === 'noMatches') return t('connections.explore.noMatches');
  switch (status.value) {
    case 'loading':
      return t('connections.explore.loading');
    case 'error':
      return t('connections.explore.error', { message: error.value });
    case 'auth_required':
      return t('connections.explore.authRequired');
    case 'rate_limited':
      return t('connections.explore.rateLimited');
    case 'ready':
      return rooms.value.length ? '' : t('connections.explore.noMatches');
    default:
      return '';
  }
});
const retryable = computed(() => status.value === 'error' || status.value === 'rate_limited');
</script>

<template>
  <BasePanel
    class="live-explorer"
    :title="t('connections.explore.title')"
    flush
  >
    <template #actions>
      <template v-if="open">
        <BaseTextField
          :model-value="keyword"
          class="live-explorer__search"
          type="search"
          size="sm"
          :placeholder="t('connections.explore.search')"
          :aria-label="t('connections.explore.search')"
          @update:model-value="store.setKeyword"
        >
          <template #prefix>
            <IconSearch />
          </template>
        </BaseTextField>
        <BaseIconButton
          :label="t('connections.explore.refresh')"
          size="sm"
          @click="store.refresh()"
        >
          <IconRefresh />
        </BaseIconButton>
        <BaseIconButton
          :label="t('connections.explore.random')"
          size="sm"
          :disabled="status === 'loading' || !rooms.length"
          @click="store.pickRandom(props.currentCreator)"
        >
          <IconDice />
        </BaseIconButton>
      </template>
      <BaseButton
        v-if="open"
        size="sm"
        pressed
        @click="store.setOpen(false)"
      >
        {{ t('connections.explore.hide') }}
      </BaseButton>
    </template>

    <div
      v-if="!open"
      class="live-explorer__closed"
    >
      <BaseButton
        :title="t('connections.explore.closedHint')"
        @click="store.setOpen(true)"
      >
        <template #icon>
          <IconSearch />
        </template>
        {{ t('connections.explore.open') }}
      </BaseButton>
    </div>
    <template v-else>
      <div
        v-if="randomCandidate"
        class="live-explorer__random"
        role="status"
      >
        <span class="live-explorer__random-label">{{ t('connections.explore.randomPreview') }}</span>
        <strong>@{{ randomCandidate.uniqueId }}</strong>
        <span class="live-explorer__spacer" />
        <BaseButton
          size="sm"
          variant="primary"
          @click="emit('connect', randomCandidate)"
        >
          {{ t('connection.connect') }}
        </BaseButton>
        <BaseButton
          size="sm"
          variant="ghost"
          @click="store.clearRandom()"
        >
          {{ t('connections.explore.dismiss') }}
        </BaseButton>
      </div>
      <DiagnosticStatusAction
        v-if="status === 'error'"
        :label="t('diagnostics.discoveryFailed')"
        source="discovery"
      />
      <p
        v-else-if="statusMessage"
        class="live-explorer__status"
        role="status"
      >
        {{ statusMessage }}
        <BaseButton
          v-if="retryable"
          size="sm"
          @click="store.refresh()"
        >
          {{ t('connections.explore.retry') }}
        </BaseButton>
      </p>
      <BaseTable
        v-if="rooms.length"
        class="live-explorer__table"
        :columns="columns"
        :rows="rooms"
        :row-key="(room: LiveChannel) => room.roomId || room.uniqueId"
      >
        <template #cell-uniqueId="{ row }">
          @{{ row.uniqueId }}
        </template>
        <template #cell-title="{ row }">
          <span
            class="live-explorer__title"
            :title="row.title"
          >{{ row.title || '—' }}</span>
        </template>
        <template #cell-viewers="{ row }">
          {{ row.viewers > 0 ? formatCount(row.viewers, locale) : '—' }}
        </template>
        <template #cell-connect="{ row }">
          <BaseButton
            size="sm"
            @click="emit('connect', row)"
          >
            {{ t('connection.connect') }}
          </BaseButton>
        </template>
      </BaseTable>
    </template>

    <template
      v-if="open && nextOffset !== null"
      #footer
    >
      <BaseButton
        size="sm"
        block
        :disabled="status === 'loading'"
        @click="store.loadMore()"
      >
        {{ t('connections.explore.loadMore') }}
      </BaseButton>
    </template>
  </BasePanel>
</template>

<style scoped>
.live-explorer__search {
  width: 200px;
}

.live-explorer__random {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-raised);
}

.live-explorer__random-label {
  color: var(--color-text-muted);
}

.live-explorer__spacer {
  flex: 1;
}

.live-explorer__status {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  color: var(--color-text-muted);
}

.live-explorer__closed {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 160px;
}

.live-explorer__table {
  height: 100%;
}

.live-explorer__title {
  color: var(--color-text-muted);
}
</style>
