import { describe, expect, it } from 'vitest';
import { viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';
import { parseGrant } from '@/entities/collaboration/lib/grants.ts';
import { areaById, canSetLevel, levelOf, levelsFor, scopeOf, withLevel, withScope } from './access-areas.ts';
import { presetGrant } from './presets.ts';

describe('access levels', () => {
  it('reads none, view, manage and custom from the grant', () => {
    const points = areaById('points');
    expect(levelOf(viewerGrant(), points)).toBe('none');
    expect(levelOf({ ...viewerGrant(), capabilities: ['points.read'] }, points)).toBe('view');
    expect(levelOf({ ...viewerGrant(), capabilities: ['points.read', 'points.adjust', 'commands.read'] }, points)).toBe('manage');
    // Only some change permissions: never shown as a clean level.
    expect(levelOf({ ...viewerGrant(), capabilities: ['workflows.read', 'workflows.toggle'] }, areaById('workflows'))).toBe('custom');
  });
  it('manage adds the read and recovery permissions that a change needs, and the result is a valid grant', () => {
    const next = withLevel(viewerGrant(), areaById('points'), 'manage');
    expect(next.capabilities).toEqual(expect.arrayContaining(['live.read', 'points.read', 'points.adjust', 'commands.read']));
    expect(parseGrant(next)).toEqual(next);
  });
  it('lowering an area removes its own permissions but nothing else, including effects and other scopes', () => {
    const base: Grant = { ...viewerGrant(), capabilities: ['live.read', 'workflows.read', 'workflows.toggle', 'workflows.edit', 'commands.read', 'http.request'], resources: { workflow: ['a'] }, maxPointsDelta: null,
      effects: { http: [], audio: [], emitTopics: [] } };
    const next = withLevel(base, areaById('workflows'), 'none');
    expect(next.capabilities).toEqual(['live.read', 'commands.read', 'http.request']);
    expect(next.effects).toEqual(base.effects);
    expect(base.capabilities).toContain('workflows.read');
  });
  it('drops the points limit when points can no longer be adjusted', () => {
    const base = withLevel(viewerGrant(), areaById('points'), 'manage'); base.maxPointsDelta = 500;
    expect(withLevel(base, areaById('points'), 'view').maxPointsDelta).toBeNull();
    expect(withLevel(base, areaById('points'), 'manage').maxPointsDelta).toBe(500);
  });
  it('never lowers a permission that automation delegation still needs', () => {
    const base: Grant = { ...viewerGrant(), capabilities: ['live.read', 'automation.read', 'automation.edit', 'automation.toggle', 'points.read', 'points.adjust', 'commands.read'], automationDelegation: { effects: ['points'], allowDynamicTargets: false } };
    expect(canSetLevel(base, areaById('automations'), 'none')).toBe(false);
    expect(canSetLevel(base, areaById('points'), 'view')).toBe(false);
    expect(canSetLevel(base, areaById('competitions'), 'manage')).toBe(true);
    expect(parseGrant(withLevel(base, areaById('automations'), 'none'))).toBeTruthy();
  });
  it('offers manage only where a change permission exists', () => {
    expect(levelsFor(areaById('live'))).toEqual(['none', 'view']);
    expect(levelsFor(areaById('workflows'))).toEqual(['none', 'view', 'manage']);
  });
  it('keeps deny-all distinct from unrestricted', () => {
    const area = areaById('workflows');
    expect(scopeOf(viewerGrant(), area)).toBeUndefined();
    const none = withScope(viewerGrant(), area, []);
    expect(scopeOf(none, area)).toEqual([]);
    expect(scopeOf(withScope(none, area, undefined), area)).toBeUndefined();
  });
  it('reviewer presets stay read-only and start with no workflows listed', () => {
    const workflow = presetGrant('workflow');
    expect(levelOf(workflow, areaById('workflows'))).toBe('view');
    expect(workflow.resources.workflow).toEqual([]);
    expect(presetGrant('agent').capabilities).toContain('commands.read');
    expect(presetGrant('viewer').capabilities).toEqual(['live.read']);
  });
});
