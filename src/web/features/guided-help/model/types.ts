import type { Router } from 'vue-router';
import type { Locale } from '@/shared/i18n/supported-locales.ts';

export type TourId = 'connection' | 'automationEvents' | 'automationActions' | 'automationVariables' | 'plugin' | 'overlay' | 'points' | 'changes' | 'diagnostics' | 'feed' | 'backup'
  | 'analytics' | 'settings' | 'overlayObs' | 'cloudOverlays' | 'rankings' | 'pluginSetup'
  | 'connectionRecovery' | 'feedInspector' | 'backupRestore'
  | 'automationEventChat' | 'automationEventGift' | 'automationEventSocial' | 'automationEventRoom'
  | 'automationActionMedia' | 'automationActionTts' | 'automationActionHttp' | 'automationActionVars'
  | 'automationFilters' | 'automationCooldowns' | 'automationTemplates';
export type TourLocale = Locale;
export interface GuideStep { id: string; selector: string; title: string; description: string }
export interface TourStepDefinition {
  id: string;
  version: number;
  target: string;
  titleKey: string;
  descriptionKey: string;
  /** Discovery methods, never permission grants or an instruction to execute. */
  requiredCapabilities: readonly string[];
  missingTargetPolicy: 'skip' | 'cancel';
  /** Selectors tried in order when the target is missing: the first visible one gets clicked so the target appears (open a record, switch a tab). */
  prepareClick?: readonly string[];
}
export interface TourDefinition {
  id: TourId;
  version: number;
  route: string;
  scope: string;
  /** External task key for action-oriented tours. Informational walkthroughs leave it undefined and complete when their final step finishes. */
  task?: string;
  steps: readonly TourStepDefinition[];
}
export type TourCapabilities = readonly string[] | { supports(method: string): boolean | undefined };
export type TourTaskState = Readonly<Record<string, boolean | undefined>>;
export type GuideOutcome = 'completed' | 'dismissed' | 'navigation' | 'replaced' | 'unavailable' | 'task-completed' | 'disposed';
export interface TourProgress {
  version: number;
  stepId: string;
  status: 'in-progress' | 'dismissed' | 'completed';
}
export interface StartTourOptions {
  router?: Router;
  capabilities?: TourCapabilities;
  /** Observed host state only. Advancing/dismissing help never sets these task flags. */
  taskState?: TourTaskState | (() => TourTaskState);
  locale?: TourLocale;
  /** A demo frame or native workspace can constrain all targets to its own root. */
  scope?: Element;
  trigger?: HTMLElement;
  /** Always capped by the coordinator; missing targets never wait indefinitely. */
  waitMs?: number;
}
export interface GuideSession {
  readonly id: string;
  readonly ready: Promise<boolean>;
  readonly done: Promise<GuideOutcome>;
  readonly active: boolean;
  stop(reason?: GuideOutcome): void;
  refresh(): void;
  updateTaskState(state: TourTaskState): void;
}
