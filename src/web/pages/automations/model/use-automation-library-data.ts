import { computed, type Ref } from 'vue';
import type { ViewerRecord } from '../../../../shared/messages.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import { availableActionTypeIds } from '@/entities/automation/lib/automation-records.ts';
import { resolvedPacks } from '@/entities/rule-template/lib/rule-profiles.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';

export function useAutomationLibraryData(automation: AutomationControls, leaderboard: Readonly<Ref<readonly ViewerRecord[]>>) {
  const { t } = useAutomationText();
  const snapshot = automation.snapshot;
  const eventTypes = computed(() => snapshot.value.eventTypes ?? []);
  const packs = computed(() =>
    resolvedPacks(
      [...automation.profilePacks.value],
      snapshot.value.events.map((event) => event.id),
      snapshot.value.actions.map((action) => action.id),
    ),
  );
  const activePack = computed(() => packs.value.find((pack) => pack.id === automation.activeProfileId.value) ?? null);
  const events = computed(() => {
    const pack = activePack.value;
    if (!pack) return snapshot.value.events;
    const ids = new Set(pack.eventIds);
    return snapshot.value.events.filter((event) => ids.has(event.id));
  });
  const actions = computed(() => {
    const pack = activePack.value;
    if (!pack) return snapshot.value.actions;
    const ids = new Set(pack.actionIds);
    return snapshot.value.actions.filter((action) => ids.has(action.id));
  });
  const availableTypes = computed(() => availableActionTypeIds(snapshot.value.plugins, snapshot.value.actionTypes));
  const gifts = computed<SearchOption[]>(() =>
    automation.gifts.value.map((gift) => ({
      value: gift.name,
      label: gift.name,
      detail: `${gift.diamondCount} 💎`,
      iconUrl: gift.iconUrl,
    })),
  );
  const viewers = computed<SearchOption[]>(() =>
    leaderboard.value.map((viewer) => ({
      value: viewer.uniqueId,
      label: viewer.nickname ? `${viewer.nickname} (@${viewer.uniqueId})` : `@${viewer.uniqueId}`,
      detail: t('automations.pickers.points', { points: viewer.points }),
    })),
  );


  return { packs, events, actions, eventTypes, availableTypes, gifts, viewers };
}
