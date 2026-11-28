/**
 * Tour modules: one file per tour colocating metadata, layout and copy for
 * every locale. `defineTour` validates the shape at type level; the loader in
 * tours/index.ts assembles the same registry/catalog structures the
 * coordinator, catalog dialog and tests already consume.
 */
import { guideTargetSelector } from './guide-dom.ts';
import { guideAction } from './guide-vocabulary.ts';
import { guideTargetMetadata } from './guide-targets.ts';
import type { TourAvailability, TourDefinition, TourRequirements, TourStepDefinition } from './types.ts';
import { automationActionExampleGuideAction } from './tour-requirements.ts';

/** Driver.js chrome copy per locale, recovered from the pre-module catalogs. */
export const controlCopy = {
  es: {
    next: 'Siguiente', previous: 'Anterior', done: 'Terminar guía', close: 'Cerrar guía', progress: '{{current}} de {{total}}',
    moved: 'Cambiamos a: {{section}}', back: 'Volvemos a: {{section}}', skipped: 'Omitimos {{count}} paso(s) que no aplican a tu configuración.',
    zones: { toolbar: 'Barra de herramientas', tabs: 'Pestañas', list: 'Lista', editor: 'Editor', preview: 'Vista previa', detail: 'Panel de detalle', footer: 'Pie de página' },
  },
  en: {
    next: 'Next', previous: 'Previous', done: 'Finish guide', close: 'Close guide', progress: '{{current}} of {{total}}',
    moved: 'Now in: {{section}}', back: 'Back to: {{section}}', skipped: 'Skipped {{count}} step(s) that do not apply to your setup.',
    zones: { toolbar: 'Toolbar', tabs: 'Tabs', list: 'List', editor: 'Editor', preview: 'Preview', detail: 'Detail panel', footer: 'Footer' },
  },
  pt: {
    next: 'Próximo', previous: 'Anterior', done: 'Concluir guia', close: 'Fechar guia', progress: '{{current}} de {{total}}',
    moved: 'Agora em: {{section}}', back: 'Voltamos a: {{section}}', skipped: 'Ignoramos {{count}} passo(s) que não se aplicam à sua configuração.',
    zones: { toolbar: 'Barra de ferramentas', tabs: 'Abas', list: 'Lista', editor: 'Editor', preview: 'Pré-visualização', detail: 'Painel de detalhes', footer: 'Rodapé' },
  },
  fr: {
    next: 'Suivant', previous: 'Précédent', done: 'Terminer le guide', close: 'Fermer le guide', progress: '{{current}} sur {{total}}',
    moved: 'Maintenant dans : {{section}}', back: 'Retour à : {{section}}', skipped: '{{count}} étape(s) ignorée(s) : elles ne concernent pas votre configuration.',
    zones: { toolbar: 'Barre d’outils', tabs: 'Onglets', list: 'Liste', editor: 'Éditeur', preview: 'Aperçu', detail: 'Panneau de détails', footer: 'Pied de page' },
  },
  ja: {
    next: '次へ', previous: '前へ', done: 'ガイドを完了', close: 'ガイドを閉じる', progress: '{{total}}件中{{current}}件目',
    moved: '次の場所に移動しました: {{section}}', back: '前の場所に戻りました: {{section}}', skipped: '設定に当てはまらない{{count}}件のステップを省略しました。',
    zones: { toolbar: 'ツールバー', tabs: 'タブ', list: '一覧', editor: 'エディター', preview: 'プレビュー', detail: '詳細パネル', footer: 'フッター' },
  },
  zh: {
    next: '下一步', previous: '上一步', done: '完成指南', close: '关闭指南', progress: '第 {{current}} 步，共 {{total}} 步',
    moved: '现在位于：{{section}}', back: '返回到：{{section}}', skipped: '已跳过 {{count}} 个不适用于你当前设置的步骤。',
    zones: { toolbar: '工具栏', tabs: '标签页', list: '列表', editor: '编辑器', preview: '预览', detail: '详情面板', footer: '页脚' },
  },
} as const;

// Catalog order, not supportedLocales order: built catalog key order follows it.
export const tourLocales = ['es', 'en', 'pt', 'fr', 'ja', 'zh'] as const;
export type TourCopyLocale = (typeof tourLocales)[number];

/** [title, description] per locale. Tuples keep six-locale copy compact. */
export type StepCopy = Readonly<Record<TourCopyLocale, readonly [string, string]>>;

/** Automation-editor selectors shared by the tour layouts and their tests. */
export const automationSelector = {
  editorClose: guideTargetSelector('automation-editor-close'),
  tabEvents: guideTargetSelector('automation-tab-events'),
  tabActions: guideTargetSelector('automation-tab-actions'),
  firstEventRow: `${guideTargetSelector('automation-events')} tbody tr`,
  firstActionRow: `${guideTargetSelector('automation-actions')} tbody tr`,
} as const;

interface LayoutStep {
  id: string;
  target: string;
  policy: 'skip' | 'cancel';
  prepare?: readonly string[];
  prepareActions?: readonly string[];
  capabilities?: readonly string[];
  version?: number;
}

/** The shared skeleton for trigger-focused automation tours. */
function eventShape(extra: readonly string[] = []): readonly LayoutStep[] {
  const { editorClose, tabEvents, firstEventRow } = automationSelector;
  const selectEventIds = [guideAction.automation.selectEvents];
  const openEventIds = [guideAction.automation.openEvent];
  const openEventPrepare = [editorClose, tabEvents, firstEventRow];
  const steps: LayoutStep[] = [
    { id: 'eventsTab', target: 'automation-tab-events', policy: 'cancel', prepare: [editorClose], prepareActions: selectEventIds },
    { id: 'events', target: 'automation-events', policy: 'skip', prepare: [editorClose, tabEvents], prepareActions: selectEventIds },
    { id: 'trigger', target: 'event-editor-trigger', policy: 'skip', prepare: openEventPrepare, prepareActions: openEventIds },
    { id: 'conditions', target: 'event-editor-conditions', policy: 'skip', prepare: openEventPrepare, prepareActions: openEventIds },
  ];
  if (extra.includes('cooldown')) steps.push({ id: 'cooldown', target: 'event-editor-cooldown', policy: 'skip', prepare: openEventPrepare, prepareActions: openEventIds });
  return steps;
}

/** The shared skeleton for effect-focused automation tours. */
function actionShape(requirements?: TourRequirements): readonly LayoutStep[] {
  const { editorClose, tabActions, firstActionRow } = automationSelector;
  const example = requirements?.automationActionExample;
  const type = requirements?.automationActionType;
  // Edit/example tours open an existing compatible record. Type-owned builtin
  // tours may open an unsaved draft of the exact declared type; draft creation
  // is still user-visible and never submitted by the guide.
  const openAction = example ? automationActionExampleGuideAction(example)
    : type?.typeId ? `automation.openOrCreateActionType:${type.typeId}`
    : type?.tag ? `automation.openActionTag:${type.tag}`
    : guideAction.automation.openAction;
  const specialized = !!(example || type);
  // A specialized tour must never select whichever first action happens to
  // exist. Only the generic actions overview keeps the first-row DOM fallback.
  const openFallback = specialized ? [editorClose, tabActions] : [editorClose, tabActions, firstActionRow];
  const editorPolicy = specialized ? 'cancel' as const : 'skip' as const;
  const selectActionIds = [guideAction.automation.selectActions];
  const openActionIds = [openAction];
  return [
    { id: 'actionsTab', target: 'automation-tab-actions', policy: 'cancel', prepare: [editorClose], prepareActions: selectActionIds },
    { id: 'actions', target: 'automation-actions', policy: 'skip', prepare: [editorClose, tabActions], prepareActions: selectActionIds },
    { id: 'settings', target: 'action-editor-settings', policy: editorPolicy, prepare: openFallback, prepareActions: openActionIds },
    { id: 'permissions', target: 'action-editor-permissions', policy: editorPolicy, prepare: openFallback, prepareActions: openActionIds },
  ];
}

export interface TourModule {
  id: string;
  version: number;
  route: string;
  scope: string;
  task?: string;
  tags: readonly string[];
  icon: string;
  /** 'event'/'action' expand to shared skeletons; 'custom' uses `layout` verbatim. */
  shape: 'event' | 'action' | 'custom';
  /** Public tours appear in the catalog; internal tours stay registered for future/direct use only. */
  lifecycle?: 'public' | 'internal';
  /** False when the signed-out demo cannot honestly represent the required account/workspace state. */
  demo?: boolean;
  availability?: TourAvailability;
  requirements?: TourRequirements;
  /** Extra steps for the event shape (today only 'cooldown'). */
  extras?: readonly string[];
  layout?: readonly LayoutStep[];
  /** Step copy keyed by step id; must cover every layout step after expansion. */
  steps: Readonly<Record<string, StepCopy>>;
  card: StepCopy;
}

export function defineTour<const T extends TourModule>(module: T): T {
  return module;
}

/** Expand a module into its effective layout. */
export function tourLayout(module: TourModule): readonly LayoutStep[] {
  if (module.shape === 'event') return eventShape(module.extras);
  if (module.shape === 'action') return actionShape(module.requirements);
  return module.layout ?? [];
}

/** Build the registry entry for a module. */
export function registryEntry(module: TourModule): TourDefinition {
  const steps: TourStepDefinition[] = tourLayout(module).map((step) => ({
    id: step.id,
    version: step.version ?? 1,
    target: step.target,
    titleKey: `${module.id}.${step.id}.title`,
    descriptionKey: `${module.id}.${step.id}.description`,
    requiredCapabilities: step.capabilities ?? [],
    missingTargetPolicy: step.policy,
    ...(step.prepare ? { prepareClick: step.prepare } : {}),
    ...(step.prepareActions ? { prepareActions: step.prepareActions } : {}),
    ...(() => {
      const placement = guideTargetMetadata(step.target);
      return {
        ...(placement.side ? { popoverSide: placement.side } : {}),
        ...(placement.align ? { popoverAlign: placement.align } : {}),
      };
    })(),
  }));
  return {
    id: module.id as TourDefinition['id'],
    version: module.version,
    route: module.route,
    scope: module.scope,
    ...(module.lifecycle ? { lifecycle: module.lifecycle } : {}),
    ...(module.availability ? { availability: module.availability } : {}),
    ...(module.requirements ? { requirements: module.requirements } : {}),
    ...(module.task ? { task: module.task } : {}),
    steps,
  } as TourDefinition;
}

type CatalogStepText = { title: string; description: string };
export type MergedCatalog = Readonly<Record<string, Readonly<Record<string, CatalogStepText>>>> & { card: Readonly<Record<string, CatalogStepText>> };

/** Build the per-locale catalog section for a set of modules. */
export function mergeCatalog(modules: readonly TourModule[], locale: TourCopyLocale): MergedCatalog {
  const out: Record<string, Record<string, CatalogStepText>> = {};
  const card: Record<string, CatalogStepText> = {};
  for (const module of modules) {
    const section: Record<string, CatalogStepText> = {};
    for (const layoutStep of tourLayout(module)) {
      const copy = module.steps[layoutStep.id];
      if (!copy) throw new Error(`tour ${module.id}: missing copy for step ${layoutStep.id}`);
      const text = copy[locale];
      if (!text) throw new Error(`tour ${module.id}/${layoutStep.id}: missing ${locale} copy`);
      section[layoutStep.id] = { title: text[0], description: text[1] };
    }
    out[module.id] = section;
    const cardText = module.card[locale];
    if (!cardText) throw new Error(`tour ${module.id}: missing ${locale} card copy`);
    card[module.id] = { title: cardText[0], description: cardText[1] };
  }
  return Object.assign(out, { card, controls: controlCopy[locale] }) as MergedCatalog;
}
