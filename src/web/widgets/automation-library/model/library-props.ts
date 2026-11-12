import type { ActionTypeDefinition, BehaviorRun, LiveAction, LiveEvent, PluginEventType } from '../../../../automation/behavior/types.ts';
import type { LibraryExtraArea } from './library-tab.ts';

export interface AutomationLibraryProps {
  events: readonly LiveEvent[];
  actions: readonly LiveAction[];
  actionTypes: readonly ActionTypeDefinition[];
  availableTypes: ReadonlySet<string>;
  eventTypes: readonly PluginEventType[];
  runs: readonly BehaviorRun[];
  /** Extra areas shown after Events and Actions (their content comes through the slot of the same name). */
  areas?: readonly LibraryExtraArea[];
}
