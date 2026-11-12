<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Grant } from '@/entities/collaboration/api/types.ts';
import { readCapabilityIds, writeCapabilityIds, effectCaps } from '../model/grant-options.ts';

const props = defineProps<{ grant: Grant }>();
const { t } = useI18n();
const count = (ids: readonly string[]): number => ids.filter(id => (props.grant.capabilities as string[]).includes(id)).length;
// Short, scannable summary. The exact grant stays one click away in the disclosure beside it.
const chips = computed(() => {
  const g = props.grant;
  const list: Array<{ key: string; label: string; tone: 'read' | 'control' | 'effect' | 'scope' | 'muted' }> = [];
  const read = count(readCapabilityIds), control = count(writeCapabilityIds), effects = count(effectCaps);
  if (read) list.push({ key: 'read', label: t('collaboration.grantChips.read', { count: read }), tone: 'read' });
  if (control) list.push({ key: 'control', label: t('collaboration.grantChips.control', { count: control }), tone: 'control' });
  if (effects) list.push({ key: 'effects', label: t('collaboration.grantChips.effects', { count: effects }), tone: 'effect' });
  if (g.automationDelegation) list.push({ key: 'delegation', label: t('collaboration.grantChips.delegation'), tone: 'effect' });
  if (!list.length) list.push({ key: 'none', label: t('collaboration.grantChips.none'), tone: 'muted' });
  const restricted = Object.values(g.resources ?? {}).some(ids => ids !== undefined);
  list.push({ key: 'scope', label: t(restricted ? 'collaboration.grantChips.restricted' : 'collaboration.grantChips.unrestricted'), tone: 'scope' });
  return list;
});
</script>

<template>
  <ul class="grant-summary">
    <li
      v-for="chip in chips"
      :key="chip.key"
      class="grant-summary__chip"
      :class="`grant-summary__chip--${chip.tone}`"
    >
      {{ chip.label }}
    </li>
  </ul>
</template>
