import type { LiveEvent, LiveAction } from '../../../../automation/behavior/types.ts';

export type EditingRecord =
  | { kind: 'event'; record: LiveEvent; isNew: boolean }
  | { kind: 'action'; record: LiveAction; isNew: boolean };
