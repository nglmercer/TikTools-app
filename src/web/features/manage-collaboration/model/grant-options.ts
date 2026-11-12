import type { AutomationDelegation, Capability } from '@/entities/collaboration/api/types.ts';

export type DelegatedEffect = AutomationDelegation['effects'][number];
/** What each delegated automation effect needs in addition to the base automation capabilities. */
export const effectCapabilities: Partial<Record<DelegatedEffect, Capability[]>> = { points: ['points.read', 'points.adjust'], http: ['http.request'], audio: ['audio.play'], emit: ['events.emit'], script: ['scripts.execute', 'live.read'] };
export const requiredAutomation: Capability[] = ['automation.read', 'automation.edit', 'automation.toggle'];
/** A change permission also needs its read permission and command recovery. */
export const writeDependencies: Partial<Record<Capability, Capability[]>> = {
  'workflows.toggle': ['workflows.read', 'commands.read'],
  'workflows.edit': ['workflows.read', 'commands.read'],
  'automation.toggle': ['automation.read', 'commands.read'],
  'automation.edit': ['automation.read', 'commands.read'],
  'points.adjust': ['points.read', 'commands.read'],
  'competitions.manage': ['competitions.read', 'commands.read'],
};
