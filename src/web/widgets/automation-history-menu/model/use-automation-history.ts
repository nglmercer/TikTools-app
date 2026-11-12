import { computed, markRaw, ref } from 'vue';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import type { AutomationControls } from '@/features/manage-automations/model/use-automation-controls.ts';
import type { ProposalTarget } from '@/features/review-automation-proposal/model/contracts.ts';
import type { MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import IconFile from '@/shared/ui/icon/icons/IconFile.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconLock from '@/shared/ui/icon/icons/IconLock.vue';

export function useAutomationHistory(controls: Pick<AutomationControls, 'persistent' | 'sourceHost'>, getTarget: () => ProposalTarget | null) {
  const { t } = useAutomationText();
  const libraryPanel = ref<{ openHistory: () => void; busy: boolean; hasPending: boolean } | null>(null);
  const proposalReviewPanel = ref<{ openReview: () => void; openHistory: () => void; busy: boolean; hasPending: boolean } | null>(null);
  const approvalReviewPanel = ref<{ openReview: () => void; openHistory: () => void; busy: boolean; hasPending: boolean } | null>(null);
  const definitionRecoveryPanel = ref<{ openHistory: () => void; busy: boolean; hasPending: boolean } | null>(null);
  // One menu for every host history and review; each entry opens the existing dialog of its panel.
  const historyItems = computed<MenuEntry[]>(() => {
    const persistent = controls.persistent.value;
    const entry = (id: string, label: string, icon: typeof IconFile, panel: { busy: boolean } | null): MenuEntry =>
      ({ id, label, icon: markRaw(icon), disabled: !panel || panel.busy });
    const items: MenuEntry[] = [];
    const proposal = proposalReviewPanel.value;
    const definition = definitionRecoveryPanel.value;
    const library = libraryPanel.value;
    const approval = approvalReviewPanel.value;
    if (persistent === true) items.push(entry('adoption', t(proposal?.hasPending ? 'automationProposal.pendingButton' : 'automationProposal.history'), IconFile, proposal));
    if (persistent === true || controls.sourceHost.value !== null) items.push(entry('definition', t(definition?.hasPending ? 'definitionRecovery.pendingButton' : 'definitionRecovery.history'), IconFlow, definition));
    if (persistent !== false) items.push(entry('library', t(library?.hasPending ? 'libraryRecovery.pendingButton' : 'libraryRecovery.history'), IconLayers, library));
    if (persistent === true) items.push(entry('approval', t(approval?.hasPending ? 'automationApproval.pendingButton' : 'automationApproval.history'), IconCheck, approval));
    if (persistent === true && getTarget()) {
      items.push({ type: 'separator', id: 'reviews' });
      items.push(entry('review-authority', t('automationProposal.open'), IconSearch, proposal));
      items.push(entry('review-approval', t('automationApproval.open'), IconLock, approval));
    }
    return items;
  });
  const historyAttention = computed(() =>
    Boolean(proposalReviewPanel.value?.hasPending || definitionRecoveryPanel.value?.hasPending || libraryPanel.value?.hasPending || approvalReviewPanel.value?.hasPending),
  );
  function openHistory(id: string): void {
    const open: Record<string, (() => void) | undefined> = {
      adoption: () => proposalReviewPanel.value?.openHistory(),
      definition: () => definitionRecoveryPanel.value?.openHistory(),
      library: () => libraryPanel.value?.openHistory(),
      approval: () => approvalReviewPanel.value?.openHistory(),
      'review-authority': () => proposalReviewPanel.value?.openReview(),
      'review-approval': () => approvalReviewPanel.value?.openReview(),
    };
    open[id]?.();
  }

  return { libraryPanel, proposalReviewPanel, definitionRecoveryPanel, approvalReviewPanel, historyItems, historyAttention, openHistory };
}
