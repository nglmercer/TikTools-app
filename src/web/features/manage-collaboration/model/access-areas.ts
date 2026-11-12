import type { Capability, Grant, ResourceKind } from '@/entities/collaboration/api/types.ts';
import { effectCapabilities, requiredAutomation, writeDependencies } from './grant-options.ts';
import { cloneGrant } from './clone-grant.ts';

/** One column of the access matrix: a read capability, the capabilities that make it "manage", and the resource a scope narrows. */
export interface AccessArea { id: 'live' | 'workflows' | 'automations' | 'points' | 'competitions'; read: Capability[]; write: Capability[]; resource: ResourceKind }
export const accessAreas: readonly AccessArea[] = [
  { id: 'live', read: ['live.read'], write: [], resource: 'live' },
  { id: 'workflows', read: ['workflows.read'], write: ['workflows.toggle', 'workflows.edit'], resource: 'workflow' },
  { id: 'automations', read: ['automation.read'], write: ['automation.toggle', 'automation.edit'], resource: 'automation' },
  { id: 'points', read: ['points.read'], write: ['points.adjust'], resource: 'viewer' },
  { id: 'competitions', read: ['competitions.read'], write: ['competitions.manage'], resource: 'competition' },
];
export type AccessLevel = 'none' | 'view' | 'manage';
/** `custom` is only ever read from an existing grant (some, not all, change permissions); it is never chosen. */
export type DisplayLevel = AccessLevel | 'custom';
export const areaById = (id: AccessArea['id']): AccessArea => accessAreas.find(area => area.id === id)!;
export const levelsFor = (area: AccessArea): AccessLevel[] => area.write.length ? ['none', 'view', 'manage'] : ['none', 'view'];

export function levelOf(grant: Grant, area: AccessArea): DisplayLevel {
  const has = (capability: Capability) => grant.capabilities.includes(capability);
  const writes = area.write.filter(has).length;
  if (area.write.length > 0 && writes === area.write.length) return 'manage';
  if (writes > 0) return 'custom';
  return area.read.every(has) ? 'view' : 'none';
}

/** Capabilities that existing automation delegation still needs, so a cell can never silently invalidate it. */
export function lockedCapabilities(grant: Grant): Capability[] {
  const delegation = grant.automationDelegation;
  if (!delegation) return [];
  return [...requiredAutomation, ...delegation.effects.flatMap(effect => effectCapabilities[effect] ?? [])];
}

/** Returns a new grant at the requested level; everything outside this area (effects, delegation, other scopes) is preserved. */
export function withLevel(grant: Grant, area: AccessArea, level: AccessLevel): Grant {
  const next = cloneGrant(grant);
  const caps = new Set<Capability>(next.capabilities);
  [...area.read, ...area.write].forEach(capability => caps.delete(capability));
  if (level !== 'none') area.read.forEach(capability => caps.add(capability));
  if (level === 'manage') area.write.forEach((capability) => { caps.add(capability); writeDependencies[capability]?.forEach(dependency => caps.add(dependency)); });
  // A removal that delegation still depends on is kept rather than producing a grant the host would reject.
  lockedCapabilities(next).forEach((capability) => { if (grant.capabilities.includes(capability)) caps.add(capability); });
  next.capabilities = [...caps];
  if (!caps.has('points.adjust')) next.maxPointsDelta = null;
  return next;
}
export const canSetLevel = (grant: Grant, area: AccessArea, level: AccessLevel): boolean => levelOf(withLevel(grant, area, level), area) === level;

export function scopeOf(grant: Grant, area: AccessArea): string[] | undefined { return grant.resources[area.resource]; }
/** An empty list is "nothing", not "everything"; an absent key is unrestricted. This keeps deny-all distinct from all. */
export function withScope(grant: Grant, area: AccessArea, ids: string[] | undefined): Grant {
  const next = cloneGrant(grant);
  if (ids === undefined) delete next.resources[area.resource]; else next.resources[area.resource] = ids;
  return next;
}
