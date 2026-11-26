/**
 * Tour modules: one file per tour colocating metadata, layout and copy for
 * every locale. `defineTour` validates the shape at type level; the loader in
 * tours/index.ts assembles the same registry/catalog structures the
 * coordinator, catalog dialog and tests already consume.
 */
import { guideTargetMetadata } from './guide-targets.ts';
import type { AutomationActionExampleRequirement, TourAvailability, TourDefinition, TourRequirements, TourStepDefinition } from './types.ts';
import { automationActionExampleGuideAction } from './tour-requirements.ts';

/** Driver.js chrome copy per locale, recovered from the pre-module catalogs. */
export const controlCopy = {
  es: { next: 'Siguiente', previous: 'Anterior', done: 'Terminar guía', close: 'Cerrar guía', progress: '{{current}} de {{total}}' },
  en: { next: 'Next', previous: 'Previous', done: 'Finish guide', close: 'Close guide', progress: '{{current}} of {{total}}' },
  pt: { next: 'Próximo', previous: 'Anterior', done: 'Concluir guia', close: 'Fechar guia', progress: '{{current}} de {{total}}' },
  fr: { next: 'Suivant', previous: 'Précédent', done: 'Terminer le guide', close: 'Fermer le guide', progress: '{{current}} sur {{total}}' },
  ja: { next: '次へ', previous: '前へ', done: 'ガイドを完了', close: 'ガイドを閉じる', progress: '{{total}}件中{{current}}件目' },
  zh: { next: '下一步', previous: '上一步', done: '完成指南', close: '关闭指南', progress: '第 {{current}} 步，共 {{total}} 步' },
} as const;

export const tourLocales = ['es', 'en', 'pt', 'fr', 'ja', 'zh'] as const;
export type TourCopyLocale = (typeof tourLocales)[number];

/** [title, description] per locale. Tuples keep six-locale copy compact. */
export type StepCopy = Readonly<Record<TourCopyLocale, readonly [string, string]>>;

const CLOSE = '[data-guide="automation-editor-close"]';
const TAB_EVENTS = '[data-guide="automation-tab-events"]';
const TAB_ACTIONS = '[data-guide="automation-tab-actions"]';
const FIRST_EVENT_ROW = '[data-guide="automation-events"] tbody tr';
const FIRST_ACTION_ROW = '[data-guide="automation-actions"] tbody tr';

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
  const steps: LayoutStep[] = [
    { id: 'eventsTab', target: 'automation-tab-events', policy: 'cancel', prepare: [CLOSE], prepareActions: ['automation.selectEvents'] },
    { id: 'events', target: 'automation-events', policy: 'skip', prepare: [CLOSE, TAB_EVENTS], prepareActions: ['automation.selectEvents'] },
    { id: 'trigger', target: 'event-editor-trigger', policy: 'skip', prepare: [CLOSE, TAB_EVENTS, FIRST_EVENT_ROW], prepareActions: ['automation.openEvent'] },
    { id: 'conditions', target: 'event-editor-conditions', policy: 'skip', prepare: [CLOSE, TAB_EVENTS, FIRST_EVENT_ROW], prepareActions: ['automation.openEvent'] },
  ];
  if (extra.includes('cooldown')) steps.push({ id: 'cooldown', target: 'event-editor-cooldown', policy: 'skip', prepare: [CLOSE, TAB_EVENTS, FIRST_EVENT_ROW], prepareActions: ['automation.openEvent'] });
  return steps;
}

/** The shared skeleton for effect-focused automation tours. */
function actionShape(requirement?: AutomationActionExampleRequirement): readonly LayoutStep[] {
  const openAction = requirement ? automationActionExampleGuideAction(requirement) : 'automation.openAction';
  // A specialized tour must never select whichever first action happens to
  // exist. Only the generic actions overview keeps the first-row DOM fallback.
  const openFallback = requirement ? [CLOSE, TAB_ACTIONS] : [CLOSE, TAB_ACTIONS, FIRST_ACTION_ROW];
  const editorPolicy = requirement ? 'cancel' as const : 'skip' as const;
  return [
    { id: 'actionsTab', target: 'automation-tab-actions', policy: 'cancel', prepare: [CLOSE], prepareActions: ['automation.selectActions'] },
    { id: 'actions', target: 'automation-actions', policy: 'skip', prepare: [CLOSE, TAB_ACTIONS], prepareActions: ['automation.selectActions'] },
    { id: 'settings', target: 'action-editor-settings', policy: editorPolicy, prepare: openFallback, prepareActions: [openAction] },
    { id: 'permissions', target: 'action-editor-permissions', policy: editorPolicy, prepare: openFallback, prepareActions: [openAction] },
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
  if (module.shape === 'action') return actionShape(module.requirements?.automationActionExample);
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
