import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { ControllerModel } from './controller.ts';

export function useControllerWorkflowDrafts(model: Pick<ControllerModel, 'rows' | 'ready' | 'draftIds' | 'draft' | 'draftStorage'>) {
  const { t } = useI18n();
  const { rows, ready } = model;
  const { supported: draftsSupported, error: draftError, busy: draftBusy, loading: draftLoading } = model.draftStorage;
  const draftErrorText = computed(() => {
    const code=draftError.value;if(!code)return '';
    if(['draft_conflict','draft_review_required','transport','timeout'].includes(code))return t('controller.root.draftErrors.unconfirmed');
    if(code==='draft_limit')return t('controller.root.draftErrors.limit');
    if(['stale_view','not_ready','forbidden'].includes(code))return t('controller.root.draftErrors.access');
    return t('controller.root.draftErrors.generic');
  });
  const selected = ref('');
  const row = computed(() => rows.value.find((r) => r.version.id === selected.value));
  const retainedDrafts = computed(() => model.draftIds().filter((id) => !rows.value.some((r) => r.version.id === id)));
  const draftText = computed(() => row.value ? model.draft(row.value.version.id)?.text ?? JSON.stringify(row.value.graph, null, 2) : retainedDrafts.value.includes(selected.value) ? model.draft(selected.value)?.text ?? '' : '');
  const conflict = computed(() => row.value && model.draft(row.value.version.id) && model.draft(row.value.version.id)?.revision !== row.value.version.revision);
  const savedDraft = computed(() => model.draftStorage.saved(selected.value));
  const draftMatches = computed(() => !!savedDraft.value && model.draft(selected.value)?.text === savedDraft.value.text && model.draft(selected.value)?.revision === savedDraft.value.baseRevision);
  watch([selected,ready],()=>{if(selected.value && ready.value)void model.draftStorage.load(selected.value);});
  return { selected, row, retainedDrafts, draftText, conflict, savedDraft, draftMatches, draftsSupported, draftError, draftBusy, draftLoading, draftErrorText };
}
export type ControllerWorkflowDrafts = ReturnType<typeof useControllerWorkflowDrafts>;
