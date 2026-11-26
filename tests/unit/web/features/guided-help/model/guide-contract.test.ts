import { describe, expect, it } from 'vitest';
import { demoTourScenarios } from '../../../../../../src/web/demo/tours.ts';
import { registryEntry, tourLayout } from '../../../../../../src/web/features/guided-help/model/define-tour.ts';
import { tourModules } from '../../../../../../src/web/features/guided-help/tours/index.ts';
import { guideTargetMetadata } from '../../../../../../src/web/features/guided-help/model/guide-targets.ts';
import { tourRegistry } from '../../../../../../src/web/features/guided-help/model/registry.ts';
import { tutorialEntries, tutorialTags } from '../../../../../../src/web/features/guided-help/model/tutorial-catalog.ts';
import { tourCategory } from '../../../../../../src/web/features/guided-help/model/tutorial-taxonomy.ts';

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
