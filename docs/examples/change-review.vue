<script setup lang="ts">
defineOptions({ name: 'ChangeReviewExample' });
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import ChangeReviewWorkspace from '@/features/review-shared/ui/ChangeReviewWorkspace.vue';
import type { ChangeReviewItem } from '@/features/review-shared/model/change-review.ts';
import { useMediaQuery } from '@/shared/lib/media-query.ts';

/**
 * Preview of the production ChangeReviewWorkspace with static data: the same
 * surface the library review, the automation save conflict and — in a later
 * phase — the sync panel share. Nothing here is saved or sent.
 */
type ScenarioId = 'profile' | 'save' | 'sync';

const catalogs = {
  en: {
    scenario: 'Scenario',
    scenarios: { profile: 'Profile switch', save: 'Save automation', sync: 'Sync' },
    titles: { profile: 'Switch profile · 123', save: 'Save · Invocar Pato', sync: 'Sync · Actions' },
    cancel: 'Cancel', readCurrent: 'Read current version', useCurrent: 'Use current as base', apply: 'Apply reviewed changes', save: 'Save', sync: 'Sync',
    loading: 'Loading review preview…', error: 'The review preview could not be loaded.',
    draftChanges: 'Draft changes', hostChanges: 'Host changes',
  },
  es: {
    scenario: 'Escenario',
    scenarios: { profile: 'Cambio de perfil', save: 'Guardar automatización', sync: 'Sincronización' },
    titles: { profile: 'Cambiar perfil · 123', save: 'Guardar · Invocar Pato', sync: 'Sincronizar · Acciones' },
    cancel: 'Cancelar', readCurrent: 'Leer versión actual', useCurrent: 'Usar actual como base', apply: 'Aplicar cambios revisados', save: 'Guardar', sync: 'Sincronizar',
    loading: 'Cargando vista previa…', error: 'No se pudo cargar la vista previa.',
    draftChanges: 'Cambios de tu borrador', hostChanges: 'Cambios del anfitrión',
  },
  pt: {
    scenario: 'Cenário',
    scenarios: { profile: 'Troca de perfil', save: 'Salvar automação', sync: 'Sincronização' },
    titles: { profile: 'Trocar perfil · 123', save: 'Salvar · Invocar Pato', sync: 'Sincronizar · Ações' },
    cancel: 'Cancelar', readCurrent: 'Ler versão atual', useCurrent: 'Usar atual como base', apply: 'Aplicar alterações revisadas', save: 'Salvar', sync: 'Sincronizar',
    loading: 'Carregando prévia…', error: 'Não foi possível carregar a prévia.',
    draftChanges: 'Alterações do rascunho', hostChanges: 'Alterações do host',
  },
  fr: {
    scenario: 'Scénario',
    scenarios: { profile: 'Changement de profil', save: 'Enregistrer l’automatisation', sync: 'Synchronisation' },
    titles: { profile: 'Changer de profil · 123', save: 'Enregistrer · Invocar Pato', sync: 'Synchroniser · Actions' },
    cancel: 'Annuler', readCurrent: 'Lire la version actuelle', useCurrent: 'Utiliser la version actuelle comme base', apply: 'Appliquer les changements examinés', save: 'Enregistrer', sync: 'Synchroniser',
    loading: 'Chargement de l’aperçu…', error: 'Impossible de charger l’aperçu.',
    draftChanges: 'Changements du brouillon', hostChanges: 'Changements de l’hôte',
  },
  zh: {
    scenario: '场景',
    scenarios: { profile: '切换配置文件', save: '保存自动化', sync: '同步' },
    titles: { profile: '切换配置文件 · 123', save: '保存 · Invocar Pato', sync: '同步 · 操作' },
    cancel: '取消', readCurrent: '读取当前版本', useCurrent: '以当前版本为基础', apply: '应用已审阅的更改', save: '保存', sync: '同步',
    loading: '正在加载预览…', error: '无法加载预览。',
    draftChanges: '草稿更改', hostChanges: '主机更改',
  },
  ja: {
    scenario: 'シナリオ',
    scenarios: { profile: 'プロファイル切替', save: '自動化を保存', sync: '同期' },
    titles: { profile: 'プロファイルを切替 · 123', save: '保存 · Invocar Pato', sync: '同期 · アクション' },
    cancel: 'キャンセル', readCurrent: '現在のバージョンを読む', useCurrent: '現在のバージョンを基準にする', apply: '確認済みの変更を適用', save: '保存', sync: '同期',
    loading: 'プレビューを読み込み中…', error: 'プレビューを読み込めませんでした。',
    draftChanges: '下書きの変更', hostChanges: 'ホスト側の変更',
  },
} as const;

const props = withDefaults(defineProps<{ state?: string }>(), { state: 'normal' });
const { locale } = useI18n();
const copy = computed(() => catalogs[locale.value as keyof typeof catalogs] ?? catalogs.en);
const viewportMobile = useMediaQuery('(width < 700px)');
const mobile = computed(() => props.state === 'mobile' || viewportMobile.value);

const spawnBefore = {
  config: { enemyId: 'Enemy - Duck', positionMode: 'random_player', quantity: 1 },
  enabled: false,
  id: 'act-1795190561207-960eec12',
  name: 'Invocar Pato',
  schemaVersion: 2,
  typeId: 'repo.game.spawn-enemy',
};
const spawnAfter = { ...spawnBefore, enabled: true };
const playBefore = {
  config: { cooldownSeconds: 5, volume: 70 },
  enabled: false,
  id: 'act-play-01',
  name: '/play',
  schemaVersion: 2,
  typeId: 'audio.play',
};
const playAfter = { ...playBefore, enabled: true, config: { cooldownSeconds: 3, volume: 90 } };
const sonicBefore = {
  config: { message: 'Gracias {user}', voice: 'SonicBoom' },
  enabled: true,
  id: 'act-sonic-01',
  name: 'Speak with SonicBoom',
  schemaVersion: 2,
  typeId: 'tts.speak',
};
const sonicAfter = { ...sonicBefore, config: { message: 'Gracias {user} por el regalo', voice: 'SonicBoom' } };
const minecraftGifts = {
  id: 'evt-minecraft-gifts',
  name: 'Minecraft gifts',
  enabled: true,
  schemaVersion: 2,
  trigger: 'tiktok.gift',
  actionIds: ['act-spawn-zombie', 'act-spawn-wall'],
};
const repoGifts = {
  id: 'evt-repo-gifts',
  name: 'R.E.P.O. regalos → mobs',
  enabled: true,
  schemaVersion: 2,
  trigger: 'tiktok.gift',
  actionIds: ['act-1795190561207-960eec12'],
};
const giftWall = {
  id: 'evt-gift-wall',
  name: 'Minecraft gift wall',
  enabled: true,
  schemaVersion: 2,
  trigger: 'tiktok.gift',
  actionIds: ['act-wall-01'],
};

const scenario = ref<ScenarioId>('profile');
const scenarioOptions = computed(() => [
  { value: 'profile' as const, label: copy.value.scenarios.profile },
  { value: 'save' as const, label: copy.value.scenarios.save },
  { value: 'sync' as const, label: copy.value.scenarios.sync },
]);

const profileItems = computed<ChangeReviewItem[]>(() => [
  { id: 'spawn-ducks', title: 'Invocar Pato', subtitle: 'repo.game.spawn-enemy · Enemy - Duck · #960eec12', count: 8, before: spawnBefore, after: spawnAfter },
  { id: 'play', title: '/play', subtitle: 'audio.play · comando de chat', before: playBefore, after: playAfter },
  { id: 'bang-play', title: '!play', subtitle: 'audio.play · alias de comando', before: { ...playBefore, id: 'act-bang-01', name: '!play' }, after: { ...playBefore, id: 'act-bang-01', name: '!play', enabled: true } },
  { id: 'sonic', title: 'Speak with SonicBoom', subtitle: 'tts.speak · voz del canal', before: sonicBefore, after: sonicAfter },
  { id: 'minecraft-gifts', title: 'Minecraft gifts', subtitle: 'tiktok.gift · entra al perfil 123', before: undefined, after: minecraftGifts },
  { id: 'repo-gifts', title: 'R.E.P.O. regalos → mobs', subtitle: 'tiktok.gift · sale del perfil activo', before: repoGifts, after: { ...repoGifts, enabled: false }, leaving: true },
  { id: 'gift-wall', title: 'Minecraft gift wall', subtitle: 'tiktok.gift · permanece igual', before: giftWall, after: giftWall },
]);

const saveBase = structuredClone(spawnBefore);
const saveDraft = { ...structuredClone(spawnBefore), enabled: true, config: { enemyId: 'Enemy - Duck', positionMode: 'random_player', quantity: 2 } };
const saveCurrent = { ...structuredClone(spawnBefore), enabled: true };
const saveItems = computed<ChangeReviewItem[]>(() => [
  { id: 'draft-diff', title: copy.value.draftChanges, subtitle: 'base → borrador', before: saveBase, after: saveDraft },
  { id: 'host-diff', title: copy.value.hostChanges, subtitle: 'base → anfitrión', before: saveBase, after: saveCurrent },
]);

const syncItems = computed<ChangeReviewItem[]>(() => [
  { id: 'sync-spawn', title: 'Invocar Pato', subtitle: 'local · repo.game.spawn-enemy', before: spawnBefore, after: spawnAfter },
  { id: 'sync-minecraft', title: 'Minecraft gifts', subtitle: 'Cloud · evento nuevo', before: undefined, after: minecraftGifts },
  { id: 'sync-repo', title: 'R.E.P.O. regalos → mobs', subtitle: 'local · eliminado', before: repoGifts, after: undefined },
]);

const items = computed(() => {
  if (props.state === 'empty') return [];
  if (scenario.value === 'save') return saveItems.value;
  if (scenario.value === 'sync') return syncItems.value;
  const rows = profileItems.value;
  if (props.state !== 'long') return rows;
  return rows.map((row) => row.id === 'spawn-ducks' ? { ...row, title: 'Invocar Pato con un nombre de acción muy largo para probar el ajuste visual' } : row);
});
const meta = computed(() => ({
  title: copy.value.titles[scenario.value],
  primary: scenario.value === 'save' ? copy.value.save : scenario.value === 'sync' ? copy.value.sync : copy.value.apply,
  secondary: scenario.value === 'save' ? copy.value.useCurrent : copy.value.readCurrent,
}));
const disabled = computed(() => props.state === 'disabled' || props.state === 'loading');
</script>

<template>
  <div class="change-review-example">
    <div class="change-review-example__scenario">
      <span>{{ copy.scenario }}</span>
      <BaseComboBox
        v-if="mobile"
        v-model="scenario"
        class="change-review-example__scenario-select"
        :options="scenarioOptions"
        size="sm"
        :aria-label="copy.scenario"
      />
      <BaseSegmented
        v-else
        v-model="scenario"
        :options="scenarioOptions"
        size="sm"
        :aria-label="copy.scenario"
      />
    </div>
    <section
      class="change-review-frame"
      aria-labelledby="change-review-example-title"
    >
      <header class="change-review-frame__top">
        <h2 id="change-review-example-title">
          {{ meta.title }}
        </h2>
      </header>
      <div
        v-if="props.state === 'loading'"
        class="change-review-frame__loading"
        role="status"
      >
        <span />
        <span />
        <span />
        {{ copy.loading }}
      </div>
      <BaseAlert
        v-else-if="props.state === 'error'"
        tone="error"
        :title="copy.error"
      />
      <ChangeReviewWorkspace
        v-else
        :items="items"
        :profile-filter="scenario === 'profile'"
      >
        <template #actions>
          <BaseButton :disabled="disabled">
            {{ copy.cancel }}
          </BaseButton>
          <BaseIconButton
            :label="meta.secondary"
            variant="default"
            :disabled="disabled"
          >
            <IconUndo v-if="scenario === 'save'" />
            <IconRefresh v-else />
          </BaseIconButton>
          <BaseIconButton
            :label="meta.primary"
            variant="primary"
            :disabled="disabled || !items.length"
          >
            <IconCheck />
          </BaseIconButton>
        </template>
      </ChangeReviewWorkspace>
    </section>
  </div>
</template>

<style scoped>
.change-review-example {
  min-width: 0;
}

.change-review-example__scenario {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
  overflow-x: auto;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.change-review-example__scenario :deep(.base-segmented) {
  flex: none;
}

.change-review-example__scenario-select {
  flex: 1;
  min-width: 0;
}

.change-review-frame {
  display: flex;
  flex-direction: column;
  min-width: 0;
  height: clamp(520px, calc(100vh - 330px), 780px);
  overflow: hidden;
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-bg-panel);
  box-shadow: var(--shadow-popup);
}

.change-review-frame__top {
  padding-bottom: var(--space-2);
}

.change-review-frame__top h2 {
  margin: 0;
  overflow: hidden;
  font-size: var(--font-size-lg);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.change-review-frame :deep(.change-review) {
  flex: 1;
  min-height: 0;
}

.change-review-frame__loading {
  display: grid;
  flex: 1;
  align-content: center;
  gap: var(--space-3);
  color: var(--color-text-muted);
}

.change-review-frame__loading span {
  display: block;
  height: 34px;
  border-radius: var(--radius-md);
  background: linear-gradient(90deg, var(--color-bg-control), var(--color-bg-hover), var(--color-bg-control));
}

@media (width < 700px) {
  .change-review-frame {
    height: clamp(560px, calc(100vh - 200px), 760px);
    padding: var(--space-3) var(--space-2);
  }
}
</style>
