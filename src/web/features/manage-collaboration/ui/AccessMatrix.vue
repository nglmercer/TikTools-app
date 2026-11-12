<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Grant, PairingRequest, Principal } from '@/entities/collaboration/api/types.ts';
import { accessAreas, levelOf, type AccessArea, type DisplayLevel } from '../model/access-areas.ts';
import type { Preset } from '../model/presets.ts';
import { useCollaborationPresentation } from '../lib/use-collaboration-presentation.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';

export interface MatrixSelection { target: string; area: AccessArea['id'] }
defineProps<{
  requests: PairingRequest[];
  principals: Principal[];
  /** Open connections per identity. */
  online: Record<string, number>;
  /** Unsaved edits, by identity ID. */
  drafts: Record<string, { grant: Grant }>;
  newGrant: Grant;
  selection: MatrixSelection | null;
  blocked: boolean;
  canInvite: boolean;
}>();
const emit = defineEmits<{
  select: [target: string, area: AccessArea['id']];
  preset: [kind: Preset];
  approve: [request: PairingRequest];
  reject: [request: PairingRequest];
  revoke: [principal: Principal];
  create: [];
  connection: [];
}>();
const { t } = useI18n();
const { date } = useCollaborationPresentation();
const presets: Array<{ kind: Preset; label: string }> = [
  { kind: 'viewer', label: 'collaboration.presetViewer' },
  { kind: 'workflow', label: 'collaboration.presetWorkflow' },
  { kind: 'agent', label: 'collaboration.presetAgent' },
];
const initial = (label: string): string => (label.trim()[0] ?? '?').toUpperCase();
const levelLabel = (level: DisplayLevel): string => t(`collaboration.levels.${level}`);
const areaLabel = (area: AccessArea): string => t(`collaboration.areas.${area.id}`);
</script>

<template>
  <section
    class="access-matrix"
    role="table"
    :aria-label="t('collaboration.accessTitle')"
  >
    <div
      class="access-matrix__row access-matrix__row--head"
      role="row"
    >
      <span role="columnheader">{{ t('collaboration.columnPerson') }}</span>
      <span
        v-for="area in accessAreas"
        :key="area.id"
        role="columnheader"
        class="access-matrix__area"
      >{{ areaLabel(area) }}</span>
      <span role="columnheader" />
    </div>

    <div
      v-for="request in requests"
      :key="request.requestId"
      class="access-matrix__row access-matrix__row--pending"
      role="row"
    >
      <div
        class="access-matrix__person"
        role="rowheader"
      >
        <span
          class="access-matrix__avatar"
          aria-hidden="true"
        >{{ initial(request.clientLabel) }}</span>
        <span class="access-matrix__who">
          <strong>{{ request.clientLabel }}</strong>
          <span class="access-matrix__sub access-matrix__sub--warning">{{ t('collaboration.askingToJoin') }} · {{ t('collaboration.expires', { when: date(request.expiresAt) }) }}</span>
          <span class="access-matrix__sub access-matrix__sub--id">{{ t('collaboration.requestId') }}: <code class="collab-id">{{ request.requestId }}</code></span>
        </span>
      </div>
      <div
        v-for="area in accessAreas"
        :key="area.id"
        class="access-matrix__cell"
        role="cell"
      >
        <span
          class="level level--requested"
          :class="`level--${levelOf(request.grant, area)}`"
        >
          <span class="level__area">{{ areaLabel(area) }}</span>
          <span>{{ levelLabel(levelOf(request.grant, area)) }}</span>
        </span>
      </div>
      <div
        class="access-matrix__actions"
        role="cell"
      >
        <BaseButton
          variant="primary"
          :disabled="blocked"
          @click="emit('approve', request)"
        >
          {{ t('collaboration.approve') }}
        </BaseButton>
        <BaseButton
          :disabled="blocked"
          @click="emit('reject', request)"
        >
          {{ t('collaboration.reject') }}
        </BaseButton>
      </div>
    </div>

    <div
      v-for="person in principals"
      :key="person.principalId"
      class="access-matrix__row"
      :class="{ 'access-matrix__row--unsaved': drafts[person.principalId] }"
      role="row"
    >
      <div
        class="access-matrix__person"
        role="rowheader"
      >
        <span
          class="access-matrix__avatar"
          aria-hidden="true"
        >{{ initial(person.clientLabel) }}</span>
        <span class="access-matrix__who">
          <strong>{{ person.clientLabel }}</strong>
          <span class="access-matrix__sub">
            <span
              class="access-matrix__dot"
              :class="{ 'access-matrix__dot--on': online[person.principalId] }"
              aria-hidden="true"
            />{{ online[person.principalId] ? t('collaboration.online', { count: online[person.principalId] }) : t('collaboration.offline') }} · {{ person.clientKind }}
          </span>
          <span
            v-if="drafts[person.principalId]"
            class="access-matrix__sub access-matrix__sub--warning"
          >{{ t('collaboration.unsaved') }}</span>
        </span>
      </div>
      <div
        v-for="area in accessAreas"
        :key="area.id"
        class="access-matrix__cell"
        role="cell"
      >
        <button
          type="button"
          class="level"
          :class="[`level--${levelOf((drafts[person.principalId] ?? person).grant, area)}`, { 'level--selected': selection?.target === person.principalId && selection.area === area.id }]"
          :aria-pressed="selection?.target === person.principalId && selection.area === area.id"
          :aria-label="`${person.clientLabel}, ${areaLabel(area)}: ${levelLabel(levelOf((drafts[person.principalId] ?? person).grant, area))}`"
          :disabled="blocked"
          @click="emit('select', person.principalId, area.id)"
        >
          <span class="level__area">{{ areaLabel(area) }}</span>
          <span>{{ levelLabel(levelOf((drafts[person.principalId] ?? person).grant, area)) }}</span>
        </button>
      </div>
      <div
        class="access-matrix__actions"
        role="cell"
      >
        <BaseButton
          variant="danger"
          :disabled="blocked"
          @click="emit('revoke', person)"
        >
          {{ t('collaboration.revoke') }}
        </BaseButton>
      </div>
    </div>

    <BaseEmptyState
      v-if="!requests.length && !principals.length"
      :title="t('collaboration.noPeople')"
    />

    <div
      class="access-matrix__row access-matrix__row--new"
      role="row"
    >
      <div
        class="access-matrix__person"
        role="rowheader"
      >
        <span class="access-matrix__who">
          <strong>{{ t('collaboration.newInvitation') }}</strong>
          <span
            class="access-matrix__presets"
            role="group"
            :aria-label="t('collaboration.presetsLabel')"
          >
            <button
              v-for="item in presets"
              :key="item.kind"
              type="button"
              class="access-matrix__preset"
              :disabled="blocked"
              @click="emit('preset', item.kind)"
            >{{ t(item.label) }}</button>
          </span>
        </span>
      </div>
      <div
        v-for="area in accessAreas"
        :key="area.id"
        class="access-matrix__cell"
        role="cell"
      >
        <button
          type="button"
          class="level"
          :class="[`level--${levelOf(newGrant, area)}`, { 'level--selected': selection?.target === 'new' && selection.area === area.id }]"
          :aria-pressed="selection?.target === 'new' && selection.area === area.id"
          :aria-label="`${t('collaboration.newInvitation')}, ${areaLabel(area)}: ${levelLabel(levelOf(newGrant, area))}`"
          :disabled="blocked"
          @click="emit('select', 'new', area.id)"
        >
          <span class="level__area">{{ areaLabel(area) }}</span>
          <span>{{ levelLabel(levelOf(newGrant, area)) }}</span>
        </button>
      </div>
      <div
        class="access-matrix__actions"
        role="cell"
      >
        <BaseButton
          variant="primary"
          :disabled="!canInvite"
          @click="emit('create')"
        >
          {{ t('collaboration.create') }}
        </BaseButton>
      </div>
    </div>
    <p
      v-if="!canInvite"
      class="access-matrix__note"
    >
      {{ t('collaboration.listenerRequired') }}
      <BaseButton
        size="sm"
        variant="ghost"
        @click="emit('connection')"
      >
        {{ t('collaboration.connectionSettings') }}
      </BaseButton>
    </p>
  </section>
</template>
