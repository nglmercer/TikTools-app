const t = (key: string): string => testMessage('en', key);
import { testMessage } from '@/shared/i18n/test-message.ts';
import { describe, expect, it } from 'vitest';
import { ref } from 'vue';
import type { WidgetStyle } from '../../../../../../src/widgets/sdk/template.ts';
import type { WidgetKind } from '../../../../../../src/widgets/sdk/templates.ts';
import { scalarControls, type ControlLabels } from '../../../../../../src/web/features/edit-overlay-design/lib/design-controls.ts';
import { useDesignEditor } from '../../../../../../src/web/features/edit-overlay-design/model/use-design-editor.ts';

const labels = new Proxy({}, { get: (_, key) => String(key) }) as ControlLabels;
const controls = scalarControls(labels);

function setup(kind: WidgetKind = 'follow', design: WidgetStyle = {}) {
  const source = ref<WidgetStyle>(design);
  const editor = useDesignEditor(ref(kind), source, {
    layer: (seed) => seed.field ?? seed.kind,
    newLayer: (layerKind) => layerKind,
    newText: () => t('overlays.editor.newText'),
  });
  return { editor, source };
}

describe('useDesignEditor', () => {
  it('starts clean from the saved design', () => {
    const { editor } = setup('follow', { radius: 20 });
    expect(editor.draft.value).toEqual({ radius: 20 });
    expect(editor.dirty.value).toBe(false);
    expect(editor.canUndo.value).toBe(false);
  });

  it('applies scalar controls and undoes them', () => {
    const { editor } = setup();
    editor.setScalar(controls.shadow, 'shadow', false);
    expect(editor.draft.value.shadow).toBe(false);
    expect(editor.dirty.value).toBe(true);
    editor.undo();
    expect(editor.draft.value).toEqual({});
    editor.redo();
    expect(editor.draft.value.shadow).toBe(false);
  });

  it('coalesces rapid slider changes into one undo step', () => {
    const { editor } = setup();
    editor.setScalar(controls.radius, 'radius', 10);
    editor.setScalar(controls.radius, 'radius', 12);
    editor.setScalar(controls.radius, 'radius', 14);
    editor.undo();
    expect(editor.draft.value.radius).toBeUndefined();
  });


  it('applies nested motion controls and resets them with the card section', () => {
    const { editor } = setup('follow', { motion: { preset: 'fade', enterMs: 300 } });
    editor.setScalar(controls.motionPreset, 'motionPreset', 'pop');
    editor.setScalar(controls.motionStaggerMs, 'motionStaggerMs', 80);
    expect(editor.draft.value.motion).toMatchObject({ preset: 'pop', enterMs: 300, staggerMs: 80 });
    editor.resetSection('card', controls);
    expect(editor.draft.value.motion).toBeUndefined();
  });

  it('inserts tokens at the caret and respects the length limit', () => {
    const { editor } = setup();
    const caret = editor.insertToken('message', 'name', 0, 0);
    expect(editor.textOf('message')).toBe('{{name}}just followed!');
    expect(caret).toBe(8);
    editor.setText('message', 'x'.repeat(296));
    expect(editor.insertToken('message', 'name', 0, 0)).toBe(0);
  });

  it('hides, restores and reorders text lines', () => {
    const { editor } = setup('gift');
    expect(editor.removableFields.value.has('diamonds')).toBe(true);
    editor.removeField('diamonds');
    expect(editor.hiddenFields.value).toContain('diamonds');
    expect(editor.draft.value.hiddenText).toEqual(['diamonds']);
    editor.addField('diamonds', 'top');
    expect(editor.orderedVisible.value[0]).toBe('diamonds');
    expect(editor.draft.value.hiddenText).toBeUndefined();
    editor.moveField('diamonds', 1);
    expect(editor.orderedVisible.value[1]).toBe('diamonds');
  });

  it('adds, moves and removes layers', () => {
    const { editor } = setup('follow');
    const initial = editor.layers.value.length;
    const id = editor.addLayer('text', 'top');
    expect(editor.layers.value[0]).toMatchObject({ id, kind: 'text', name: 'text 1', text: t('overlays.editor.newText') });
    editor.moveLayer(id, 1);
    expect(editor.layers.value[1]?.id).toBe(id);
    editor.editLayer(id, 'name', { name: 'Shout' });
    expect(editor.layers.value[1]?.name).toBe('Shout');
    editor.removeLayer(id);
    expect(editor.layers.value).toHaveLength(initial);
  });

  it('resets one section or the whole design', () => {
    const { editor } = setup('follow', { radius: 30, background: '#000000', textColor: '#ffffff' });
    const card = editor.schema.value.editorSections.find((section) => section.id === 'card');
    expect(card).toBeDefined();
    const keys = editor.sectionResetKeys('card', controls);
    editor.resetSection('card', controls);
    for (const key of keys) expect(Object.keys(editor.draft.value)).not.toContain(key);
    editor.resetDesign();
    expect(editor.draft.value).toEqual({});
  });

  it('reloads from the source and clears history', () => {
    const { editor, source } = setup();
    editor.setScalar(controls.shadow, 'shadow', false);
    source.value = { accent: '#123456' };
    editor.load();
    expect(editor.draft.value).toEqual({ accent: '#123456' });
    expect(editor.canUndo.value).toBe(false);
    expect(editor.dirty.value).toBe(false);
  });
});
