<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { WidgetKind } from '../../../../widgets/sdk/templates.ts';
import { overlayKinds, isOverlayKind } from '@/entities/overlay/lib/overlay-host.ts';
import { useOverlaysStore } from '@/entities/overlay/model/overlays-store.ts';
import { useCloudOverlayStore } from '@/entities/overlay/model/cloud-overlay-store.ts';
import { useCloudOverlayLibraryStore } from '@/entities/cloud-bundles/model/overlay-library-store.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import CloudBundles from '@/features/cloud-bundles/ui/CloudBundles.vue';
import CloudOverlays from '@/features/cloud-overlays/ui/CloudOverlays.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconSearch from '@/shared/ui/icon/icons/IconSearch.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import OverlayLibraryPreview from './OverlayLibraryPreview.vue';

const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ select: [kind: WidgetKind] }>();
const { t } = useI18n();
const capabilities = useCapabilityStore();
const designs = useOverlaysStore();
const cloud = useCloudOverlayStore();
const library = useCloudOverlayLibraryStore();
const section = ref<'library' | 'mine' | 'images' | 'editor'>('library');
const query = ref('');
const kind = ref('');
const sort = ref('recent');
const selected = ref<string | null>(null);
const opening = ref(false);
const overlaysAvailable = computed(() => capabilities.supports('bundles.overlays.create') === true);
const bundlesAvailable = computed(() => capabilities.supports('bundles.packages.preflight') === true);
const sections = computed(() => [
  { value: 'library' as const, label: t('overlayLibrary.library') },
  { value: 'mine' as const, label: t('overlayLibrary.mine') },
  ...(bundlesAvailable.value ? [{ value: 'images' as const, label: t('overlayLibrary.images') }] : []),
  ...(overlaysAvailable.value ? [{ value: 'editor' as const, label: t('overlayLibrary.edit') }] : []),
]);
const options = computed(() => [{ value: '', label: t('overlayLibrary.all') }, ...overlayKinds.map(value => ({ value, label: t(`overlays.kinds.${value}.name`) }))]);
const cards = computed(() => {
  const items = section.value === 'mine'
    ? library.overlays.map(item => ({ id: item.id, kind: item.kind, name: item.name, subtitle: t('overlayLibrary.revision', { revision: item.revision }), updated: item.updatedAt }))
    : overlayKinds.map(value => ({ id: value, kind: value, name: t(`overlays.kinds.${value}.name`), subtitle: t('overlayLibrary.builtin'), updated: '' }));
  const filtered = items.filter(item => (!kind.value || item.kind === kind.value) && item.name.toLocaleLowerCase().includes(query.value.trim().toLocaleLowerCase()));
  return sort.value === 'name' ? filtered.sort((a,b) => a.name.localeCompare(b.name)) : filtered.sort((a,b) => b.updated.localeCompare(a.updated));
});
const current = computed(() => cards.value.find(item => item.id === selected.value));
watch(section, value => {
  selected.value = null;
  if (value === 'mine' && library.available === true && !library.loaded) void library.refresh();
});
watch(open, value => { if (value) { section.value = 'library'; query.value = ''; kind.value = ''; selected.value = null; } });
watch(cards, value => { if (!value.some(item => item.id === selected.value)) selected.value = null; });
async function edit(): Promise<void> {
  const item = current.value;
  if (!item || opening.value) return;
  if (section.value === 'library' && isOverlayKind(item.kind)) { emit('select', item.kind); return; }
  opening.value = true;
  try { if (await cloud.load(item.id)) section.value = 'editor'; }
  finally { opening.value = false; }
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t('overlayLibrary.title')"
    :width="1120"
    class="overlay-library"
  >
    <div class="overlay-library__layout">
      <BaseComboBox
        v-model="section"
        class="overlay-library__mobile-nav"
        :options="sections"
        :aria-label="t('overlayLibrary.title')"
      />
      <nav
        class="overlay-library__sidebar"
        :aria-label="t('overlayLibrary.title')"
      >
        <BaseButton
          :pressed="section === 'library'"
          @click="section = 'library'"
        >
          <template #icon>
            <IconLayers />
          </template>{{ t('overlayLibrary.library') }}
        </BaseButton>
        <BaseButton
          :pressed="section === 'mine'"
          @click="section = 'mine'"
        >
          <template #icon>
            <IconCloud />
          </template>{{ t('overlayLibrary.mine') }}
        </BaseButton>
        <BaseButton
          v-if="bundlesAvailable"
          :pressed="section === 'images'"
          @click="section = 'images'"
        >
          <template #icon>
            <IconUpload />
          </template>{{ t('overlayLibrary.images') }}
        </BaseButton>
        <BaseButton
          v-if="overlaysAvailable"
          :pressed="section === 'editor'"
          @click="section = 'editor'"
        >
          <template #icon>
            <IconLayers />
          </template>{{ t('overlayLibrary.edit') }}
        </BaseButton>
      </nav>
      <div
        class="overlay-library__content"
        :aria-busy="opening || library.loading"
      >
        <template v-if="section === 'library' || section === 'mine'">
          <div class="overlay-library__filters">
            <BaseTextField
              v-model="query"
              type="search"
              :placeholder="t('overlayLibrary.search')"
              :aria-label="t('overlayLibrary.search')"
            >
              <template #prefix>
                <IconSearch />
              </template>
            </BaseTextField>
            <label>{{ t('overlayLibrary.kind') }}<BaseComboBox
              v-model="kind"
              :options="options"
              :aria-label="t('overlayLibrary.kind')"
            /></label>
            <label v-if="section === 'mine'">{{ t('overlayLibrary.sort') }}<BaseComboBox
              v-model="sort"
              :options="[{ value: 'recent', label: t('overlayLibrary.recent') }, { value: 'name', label: t('overlayLibrary.name') }]"
              :aria-label="t('overlayLibrary.sort')"
            /></label>
          </div>
          <BaseEmptyState
            v-if="section === 'mine' && library.available !== true"
            :title="t('overlayLibrary.unavailable')"
          />
          <BaseEmptyState
            v-else-if="section === 'mine' && library.loading && !library.loaded"
            :title="t('overlayLibrary.loading')"
          />
          <BaseEmptyState
            v-else-if="section === 'mine' && library.listFailure"
            :title="t('overlayLibrary.failed')"
          >
            <BaseButton @click="library.refresh()">
              {{ t('overlayLibrary.retry') }}
            </BaseButton>
          </BaseEmptyState>
          <template v-else>
            <BaseEmptyState
              v-if="!cards.length"
              :title="t('overlayLibrary.empty')"
            />
            <div class="overlay-library__grid">
              <button
                v-for="item in cards"
                :key="item.id"
                class="overlay-library__card"
                :aria-pressed="selected === item.id"
                :disabled="opening"
                @click="selected = item.id"
                @dblclick="edit"
              >
                <OverlayLibraryPreview
                  v-if="section === 'library' && isOverlayKind(item.kind)"
                  :kind="item.kind"
                  :design="designs.designs[item.kind]"
                  :label="t('overlayLibrary.preview', { name: item.name })"
                />
                <div
                  v-else
                  class="overlay-library__cloud-preview"
                >
                  <IconCloud :size="36" /><span>{{ item.name }}</span>
                </div>
                <span
                  v-if="selected === item.id"
                  class="overlay-library__check"
                ><IconCheck /></span>
                <span class="overlay-library__card-body"><strong>{{ item.name }}</strong><span v-if="section === 'mine'">{{ item.subtitle }}</span></span>
              </button>
            </div>
            <BaseButton
              v-if="section === 'mine' && library.hasMore"
              :disabled="library.loading"
              @click="library.loadMore()"
            >
              {{ t('overlayLibrary.more') }}
            </BaseButton>
          </template>
          <p
            v-if="cloud.error"
            role="alert"
          >
            {{ cloud.error }}
          </p>
        </template>
        <CloudBundles v-else-if="section === 'images' && bundlesAvailable" />
        <CloudOverlays v-else-if="section === 'editor' && overlaysAvailable" />
      </div>
    </div>
    <template
      v-if="section === 'library' || section === 'mine'"
      #footer-start
    >
      <div
        v-if="current"
        class="overlay-library__selection"
      >
        <strong>{{ current.name }}</strong>
      </div>
    </template>
    <template
      v-if="section === 'library' || section === 'mine'"
      #footer
    >
      <BaseButton
        v-if="section === 'library' || section === 'mine'"
        variant="primary"
        :disabled="!current || opening || (section === 'mine' && !overlaysAvailable)"
        @click="edit"
      >
        {{ t('overlayLibrary.open') }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<style scoped>
.overlay-library :deep(.base-dialog__body) { padding:0; }
.overlay-library :deep(.base-dialog__header) { padding:var(--space-6) var(--space-7); }
.overlay-library :deep(.base-dialog__title) { font-size:var(--font-size-lg); }
.overlay-library :deep(.base-dialog__footer) { border-top:1px solid var(--color-border); padding:var(--space-6) var(--space-7); }
.overlay-library__layout { display:grid; grid-template-columns:180px minmax(0,1fr); min-height:min(60vh,560px); }
.overlay-library__sidebar { display:flex; flex-direction:column; gap:var(--space-4); padding:var(--space-6); border-right:1px solid var(--color-border-subtle); }
.overlay-library__sidebar :deep(button) { justify-content:flex-start; height:auto; min-height:var(--control-height-lg); padding:var(--space-5); white-space:normal; text-align:left; background:transparent; border-color:transparent; }
.overlay-library__sidebar :deep([aria-pressed="true"]) { background:var(--color-bg-selected); border-color:var(--color-border-selected); }
.overlay-library__content { min-width:0; padding:var(--space-6) var(--space-7); background:var(--color-bg-app); }
.overlay-library__filters { display:flex; align-items:end; flex-wrap:wrap; gap:var(--space-5); margin-bottom:var(--space-6); }
.overlay-library__filters > :first-child { flex:1 1 180px; }
.overlay-library__filters label { display:grid; gap:var(--space-2); color:var(--color-text-muted); font-size:var(--font-size-sm); }
.overlay-library__grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:var(--space-6); margin-bottom:var(--space-6); }
.overlay-library__card { position:relative; display:flex; flex-direction:column; padding:0; min-width:0; overflow:hidden; border:1px solid var(--color-border); border-radius:var(--radius-md); background:var(--color-bg-panel); color:var(--color-text); font:inherit; cursor:pointer; text-align:left; }
.overlay-library__card:hover { border-color:var(--color-border-strong); }
.overlay-library__card[aria-pressed="true"] { border-color:var(--color-border-selected); box-shadow:0 0 0 1px var(--color-border-selected); }
.overlay-library__card:focus-visible { outline:2px solid var(--color-border-focus); outline-offset:3px; }
.overlay-library__card-body { display:grid; gap:var(--space-3); padding:var(--space-5); border-top:1px solid var(--color-border-subtle); }
.overlay-library__card-body > span { color:var(--color-text-muted); font-size:var(--font-size-sm); }
.overlay-library__check { position:absolute; top:var(--space-4); right:var(--space-4); display:grid; place-items:center; padding:var(--space-2); border-radius:var(--radius-md); background:var(--color-bg-selected); }
.overlay-library__cloud-preview { display:grid; place-content:center; justify-items:center; gap:var(--space-5); aspect-ratio:16/9; width:100%; color:var(--color-text-muted); }
.overlay-library__selection { display:grid; gap:var(--space-2); min-width:0; }
.overlay-library__selection span { color:var(--color-text-muted); font-size:var(--font-size-sm); }
.overlay-library__mobile-nav { display:none; }
@media (width < 900px) {
 .overlay-library__layout { grid-template-columns:minmax(0,1fr); }
 .overlay-library__sidebar { flex-direction:row; flex-wrap:wrap; border-right:0; border-bottom:1px solid var(--color-border-subtle); }
}
/* A full-height sheet leaves the content, rather than navigation, the mobile viewport. */
@media (width < 700px) {
 .overlay-library { min-width:100vw; max-width:100vw; height:100dvh; max-height:100dvh; margin:0; box-sizing:border-box; border:0; border-radius:0; }
 .overlay-library :deep(.base-dialog__header) { padding:var(--space-4) var(--space-5); }
 .overlay-library :deep(.base-dialog__body) { display:flex; flex-direction:column; overflow:hidden; }
 .overlay-library :deep(.base-dialog__footer) { padding:var(--space-5); padding-bottom:max(var(--space-5),env(safe-area-inset-bottom)); }
 .overlay-library :deep(.base-dialog__footer > .base-button) { flex:1; }
 .overlay-library__layout { display:flex; flex:1; flex-direction:column; min-height:0; }
 .overlay-library__sidebar { display:none; }
 .overlay-library__mobile-nav { display:flex; flex:none; margin:var(--space-5); }
 .overlay-library__content { flex:1; overflow:auto; padding:var(--space-5); overscroll-behavior:contain; }
 .overlay-library__filters { display:grid; grid-template-columns:minmax(0,1fr) minmax(0,1fr); gap:var(--space-4); margin-bottom:var(--space-5); }
 .overlay-library__filters > :first-child { grid-column:1 / -1; }
 .overlay-library__filters label { min-width:0; }
 .overlay-library__filters label:nth-child(2):last-child { grid-column:1 / -1; }
 .overlay-library__grid { grid-template-columns:minmax(0,1fr); }
 .overlay-library__selection { display:none; }
}
</style>
