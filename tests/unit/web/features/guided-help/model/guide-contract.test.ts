import { describe, expect, it } from 'vitest';
import { demoTourScenarios } from '../../../../../../src/web/demo/tours.ts';
import { registryEntry, tourLayout } from '../../../../../../src/web/features/guided-help/model/define-tour.ts';
import { tourModules } from '../../../../../../src/web/features/guided-help/tours/index.ts';
import { guideTargetMetadata } from '../../../../../../src/web/features/guided-help/model/guide-targets.ts';
import { tourRegistry } from '../../../../../../src/web/features/guided-help/model/registry.ts';
import { tutorialEntries, tutorialTags } from '../../../../../../src/web/features/guided-help/model/tutorial-catalog.ts';
import { tourCategory } from '../../../../../../src/web/features/guided-help/model/tutorial-taxonomy.ts';
import { automationActionExampleAvailable } from '../../../../../../src/web/features/guided-help/model/tour-requirements.ts';

describe('guide contract', () => {
  it('uses lifecycle metadata as the only public catalog boundary', () => {
    const publicIds = tourModules.filter((module) => !('lifecycle' in module) || module.lifecycle !== 'internal').map((module) => module.id);
    expect(tutorialEntries.map((entry) => entry.id).sort()).toEqual(publicIds.sort());
    expect(tutorialEntries.some((entry) => entry.id === 'rankings')).toBe(false);
    expect(tourRegistry.rankings.lifecycle).toBe('internal');
  });

  it('derives categories from product scope rather than naming conventions', () => {
    expect(tourCategory(tourModules.find((module) => module.id === 'changes')!)).toBe('automations');
    expect(tutorialEntries.find((entry) => entry.id === 'changes')?.category).toBe('automations');
    expect(tutorialEntries.find((entry) => entry.id === 'changes')?.tags).not.toContain('changes');
  });


  it('uses explicit public taxonomy tags', () => {
    const publicModules = tourModules.filter((module) => !('lifecycle' in module) || module.lifecycle !== 'internal');
    for (const module of publicModules) {
      for (const tag of module.tags) expect(tutorialTags, `${module.id}.${tag}`).toContain(tag);
    }
  });

  it('uses demo availability declared by the same tour modules', () => {
    const expected = tourModules
      .filter((module) => (!('lifecycle' in module) || module.lifecycle !== 'internal') && (!('demo' in module) || module.demo !== false))
      .map((module) => module.id)
      .sort();
    expect(Object.keys(demoTourScenarios).sort()).toEqual(expected);
    for (const id of ['backup', 'backupRestore', 'cloudOverlays', 'rankings']) {
      expect(Object.hasOwn(demoTourScenarios, id), id).toBe(false);
    }
  });


  it('declares account prerequisites in the canonical registry', () => {
    for (const id of ['backup', 'backupRestore', 'cloudOverlays'] as const) {
      expect(tourRegistry[id].availability?.account).toBe('signed-in');
    }
  });


  it('declares action requirements for type-specific tours', () => {
    expect(tourRegistry.automationActionHttp.requirements?.automationActionType).toEqual({ typeId: 'core.fetch' });
    expect(tourRegistry.automationActionMedia.requirements?.automationActionType).toEqual({ typeId: 'audio.play' });
    expect(tourRegistry.automationActionHttp.steps.find((step) => step.id === 'settings')?.prepareActions).toEqual(['automation.openOrCreateActionType:core.fetch']);
    expect(tourRegistry.automationActionMedia.steps.find((step) => step.id === 'settings')?.prepareActions).toEqual(['automation.openOrCreateActionType:audio.play']);
    expect(tourRegistry.automationActionTts.requirements?.automationActionExample).toEqual({ tag: 'tts' });
    for (const id of ['automationActionHttp', 'automationActionMedia', 'automationActionTts'] as const) {
      for (const step of tourRegistry[id].steps.filter((step) => step.id === 'settings' || step.id === 'permissions')) {
        expect(step.missingTargetPolicy, `${id}.${step.id}`).toBe('cancel');
        expect(step.prepareClick ?? [], `${id}.${step.id}`).not.toContain('[data-guide="automation-actions"] tbody tr');
      }
    }
  });


  it('requires a declared type or example for every specialized action-editor tour', () => {
    const generic = new Set(['automationActions', 'automationActionVars']);
    for (const module of tourModules.filter((module) => module.shape === 'action' && !generic.has(module.id))) {
      const requirements = tourRegistry[module.id].requirements;
      expect(requirements?.automationActionExample ?? requirements?.automationActionType, module.id).toBeTruthy();
    }
  });

  it('requires both an active action type and an existing matching record', () => {
    const fetchType = { id: 'core.fetch', tag: 'fetch', title: { default: 'Call a URL', i18key: '' }, source: { kind: 'builtin' as const } };
    const ttsType = { id: 'demo.tts', tag: 'tts', title: { default: 'Speak', i18key: '' }, source: { kind: 'plugin' as const, pluginId: 'demo.audio' } };
    const plugin = { installed: true, enabled: true, descriptor: { id: 'demo.audio' } };
    expect(automationActionExampleAvailable({ typeId: 'core.fetch' }, {
      actions: [{ typeId: 'core.fetch' }], actionTypes: [fetchType], plugins: [],
    })).toBe(true);
    expect(automationActionExampleAvailable({ typeId: 'core.fetch' }, {
      actions: [{ typeId: 'core.log' }], actionTypes: [fetchType], plugins: [],
    })).toBe(false);
    expect(automationActionExampleAvailable({ tag: 'tts' }, {
      actions: [{ typeId: 'demo.tts' }], actionTypes: [ttsType], plugins: [plugin],
    })).toBe(true);
    expect(automationActionExampleAvailable({ tag: 'tts' }, {
      actions: [{ typeId: 'demo.tts' }], actionTypes: [ttsType], plugins: [{ ...plugin, enabled: false }],
    })).toBe(false);
  });

  it('covers every registry target with explicit layout and popover metadata', () => {
    for (const module of tourModules) {
      const definition = registryEntry(module);
      for (const step of definition.steps) {
        const metadata = guideTargetMetadata(step.target);
        expect(metadata.zone, `${module.id}.${step.id}`).toBeTruthy();
        expect(step.popoverSide, `${module.id}.${step.id}`).toBe(metadata.side);
        expect(step.popoverAlign, `${module.id}.${step.id}`).toBe(metadata.align);
      }
    }
  });

  it('never prepares an automation editor through the create/type-picker control', () => {
    for (const module of tourModules) {
      for (const step of tourLayout(module)) {
        expect(step.prepare ?? [], `${module.id}.${step.id}`).not.toContain('[data-guide="automation-new-record"]');
      }
    }
  });
});
