import { describe, expect, it } from 'vitest';
import { demoTourScenarios, demoTutorialIds, scenarioTour, tourForRoute } from '../../../src/web/demo/tours.ts';

describe('demo tour recommendations', () => {
  it('offers every demo tour in catalog order', () => {
    expect(demoTutorialIds).toEqual(Object.keys(demoTourScenarios));
    expect(demoTutorialIds).not.toContain('backup');
  });

  it('recommends the tour for the section currently open', () => {
    expect(tourForRoute('/connections', 'feed')).toBe('connection');
    expect(tourForRoute('/feed', 'connection')).toBe('feed');
    expect(tourForRoute('/plugins/plugin-id/page', 'connection')).toBe('plugin');
    expect(tourForRoute('/overlays', 'connection')).toBe('overlay');
    expect(tourForRoute('/points', 'connection')).toBe('points');
    expect(tourForRoute('/debug', 'connection')).toBe('diagnostics');
    expect(tourForRoute('/automations', 'connection')).toBe('automationEvents');
    expect(tourForRoute('/automations?view=variables', 'connection')).toBe('automationVariables');
  });

  it('falls back to the scenario tour on catalog and settings pages', () => {
    expect(tourForRoute('/components/buttons', 'points')).toBe('points');
    expect(tourForRoute('/settings', 'plugins')).toBe('settings');
    expect(scenarioTour('analytics')).toBe('analytics');
  });
});
