import type { AutomationDelegation, Capability } from '@/entities/collaboration/api/types.ts';

export const readCapabilityIds = ['live.read', 'workflows.read', 'automation.read', 'points.read', 'competitions.read', 'commands.read'] as const;
export const writeCapabilityIds = ['workflows.toggle', 'workflows.edit', 'automation.toggle', 'automation.edit', 'points.adjust', 'competitions.manage'] as const;
export const current = [...readCapabilityIds, ...writeCapabilityIds] as const;
export const capabilityGroups = [
  { key: 'read', ids: readCapabilityIds },
  { key: 'write', ids: writeCapabilityIds },
] as const;
export const labels = { 'live.read': 'capLive', 'workflows.read': 'capWorkflow', 'workflows.toggle': 'capWorkflowDisable', 'workflows.edit': 'capWorkflowDelete', 'automation.read': 'capAutomation', 'automation.toggle': 'capEventDisable', 'automation.edit': 'capEventDelete', 'points.read': 'capPoints', 'points.adjust': 'capPointsAdjust', 'competitions.read': 'capCompetitions', 'competitions.manage': 'capCompetitionStop', 'commands.read': 'capCommands' } as const;
export const effectCaps = ['http.request', 'audio.play', 'events.emit', 'scripts.execute'] as const;
export const effectLabels = { 'http.request': 'capHttp', 'audio.play': 'capAudio', 'events.emit': 'capEmit', 'scripts.execute': 'capScript' } as const;
export type DelegatedEffect = AutomationDelegation['effects'][number];
export const delegatedEffects: DelegatedEffect[] = ['log', 'delay', 'points', 'http', 'audio', 'emit', 'script'];
export const effectCapabilities: Partial<Record<DelegatedEffect, Capability[]>> = { points: ['points.read', 'points.adjust'], http: ['http.request'], audio: ['audio.play'], emit: ['events.emit'], script: ['scripts.execute', 'live.read'] };
export const requiredAutomation: Capability[] = ['automation.read', 'automation.edit', 'automation.toggle'];
export const writeDependencies: Partial<Record<Capability, Capability[]>> = {
  'workflows.toggle': ['workflows.read', 'commands.read'],
  'workflows.edit': ['workflows.read', 'commands.read'],
  'automation.toggle': ['automation.read', 'commands.read'],
  'automation.edit': ['automation.read', 'commands.read'],
  'points.adjust': ['points.read', 'commands.read'],
  'competitions.manage': ['competitions.read', 'commands.read'],
};
