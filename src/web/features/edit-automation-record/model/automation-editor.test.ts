import { describe, expect, test, vi } from 'vitest';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { AutomationRecordVersion } from '@/entities/automation/model/record-versions.ts';
import { useAutomationEditor, type AutomationEditorGateway } from './automation-editor.ts';

const action: LiveAction = { id: 'same', name: 'Initial', schemaVersion: 2, enabled: true, typeId: 'core.log', config: { message: 'one' } };
const event: LiveEvent = { id: 'same', name: 'Event', schemaVersion: 1, enabled: true, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'user', runMode: 'all', actionIds: ['same'] };
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
function gateway(): AutomationEditorGateway {
  return { read: vi.fn(async () => ({ record: action, revision: '1' })),
    save: vi.fn(async (_kind, record) => ({ record, revision: '2' })), remove: vi.fn(async () => {}) };
}

describe('automation editor base and outcome fencing', () => {
  test('two editors retain their opening revision: one wins and the other preserves its candidate', async () => {
    let current = { record: action, revision: '1' };
    const host = gateway();
    host.read = vi.fn(async () => current);
    host.save = vi.fn(async (_kind, record, base) => {
      if (base.revision !== current.revision) throw new ControlCallError('revision_conflict', 'Host changed');
      current = { record: record as LiveAction, revision: '2' };
      return current;
    });
    const first = useAutomationEditor(host);
    const second = useAutomationEditor(host);
    for (const editor of [first, second]) editor.open('action', action, '1');
    first.change({ ...action, name: 'First' }); second.change({ ...action, name: 'Second' });
    await first.save(); await second.save();
    expect(current.record.name).toBe('First');
    expect(first.dirty.value).toBe(false);
    expect(second.state.value).toMatchObject({ revision: '1', candidate: { name: 'Second' }, base: { name: 'Initial' }, reviewRequired: true, failure: 'conflict' });
    await second.save();
    expect(host.save).toHaveBeenCalledTimes(2);
    await second.readForReview();
    expect(second.state.value?.candidate.name).toBe('Second');
    expect(second.adoptReview()).toBe(true);
    expect(host.save).toHaveBeenCalledTimes(2);
    expect(second.state.value?.revision).toBe('2');
    expect(second.dirty.value).toBe(true);
  });

  test('edits made during a successful save survive with the acknowledged base', async () => {
    const pending = deferred<AutomationRecordVersion>();
    const host = gateway(); host.save = vi.fn(() => pending.promise);
    const editor = useAutomationEditor(host);
    editor.open('action', action, '1'); editor.change({ ...action, name: 'Submitted' });
    const saving = editor.save();
    editor.change({ ...action, name: 'Newer draft' });
    await editor.save();
    expect(host.save).toHaveBeenCalledTimes(1);
    pending.resolve({ record: { ...action, name: 'Submitted' }, revision: '2' });
    await saving;
    expect(editor.state.value).toMatchObject({ candidate: { name: 'Newer draft' }, base: { name: 'Submitted' }, revision: '2', busy: false });
    expect(editor.dirty.value).toBe(true);
  });

  test.each(['resolve', 'reject'] as const)('a late %s cannot change a reopened same-ID editor or a different family', async (outcome) => {
    for (const reopenEvent of [false, true]) {
      const pending = deferred<AutomationRecordVersion>();
      const host = gateway(); host.save = vi.fn(() => pending.promise);
      const editor = useAutomationEditor(host);
      editor.open('action', action, '1'); editor.change({ ...action, name: 'Submitted' });
      const saving = editor.save();
      editor.close();
      editor.open(reopenEvent ? 'event' : 'action', reopenEvent ? event : action, '9');
      editor.change({ ...(reopenEvent ? event : action), name: 'Current selection' });
      if (outcome === 'resolve') pending.resolve({ record: { ...action, name: 'Submitted' }, revision: '2' });
      else pending.reject(new Error('late disconnect'));
      await saving;
      expect(editor.state.value).toMatchObject({ revision: '9', candidate: { name: 'Current selection' }, reviewRequired: false, busy: false });
    }
  });

  test('a lost save acknowledgement blocks resend; an explicit read can confirm matching current text without another write', async () => {
    const host = gateway(); const saved = { ...action, name: 'On host' };
    host.save = vi.fn(async () => { throw new ControlCallError('timeout', 'Acknowledgement lost'); });
    host.read = vi.fn(async () => ({ record: saved, revision: '2' }));
    const editor = useAutomationEditor(host);
    editor.open('action', action, '1'); editor.change(saved);
    await editor.save(); await editor.save();
    expect(host.save).toHaveBeenCalledTimes(1);
    expect(editor.state.value?.failure).toBe('unconfirmed');
    expect(editor.dirty.value).toBe(true);
    await editor.readForReview();
    expect(editor.state.value?.revision).toBe('1');
    expect(editor.adoptReview()).toBe(true);
    expect(editor.dirty.value).toBe(false);
    expect(host.save).toHaveBeenCalledTimes(1);
  });

  test('deletion requires explicit preparation against the tombstone before a separate create', async () => {
    const host = gateway();
    host.save = vi.fn(async () => { throw new ControlCallError('automation_not_found', 'Deleted'); });
    host.read = vi.fn(async () => ({ record: null, revision: '4' }));
    const editor = useAutomationEditor(host);
    editor.open('action', action, '1'); editor.change({ ...action, name: 'Kept' });
    await editor.save();
    expect(host.save).toHaveBeenCalledWith('action', expect.objectContaining({ name: 'Kept' }), { revision: '1', create: false });
    await editor.readForReview(); editor.adoptReview();
    expect(editor.state.value).toMatchObject({ base: null, revision: '4', create: true, candidate: { name: 'Kept' } });
    expect(host.save).toHaveBeenCalledTimes(1);
    host.save = vi.fn(async (_kind, record) => ({ record, revision: '5' }));
    await editor.save();
    expect(host.save).toHaveBeenCalledWith('action', expect.anything(), { revision: '4', create: true });
    expect(editor.dirty.value).toBe(false);
  });

  test('late review reads do not overwrite edits, and a changed candidate invalidates an already displayed review', async () => {
    const pending = deferred<AutomationRecordVersion>(); const host = gateway(); host.read = vi.fn(() => pending.promise);
    const editor = useAutomationEditor(host); editor.open('action', action, '1');
    const reading = editor.readForReview();
    editor.change({ ...action, name: 'Typed during read' });
    pending.resolve({ record: { ...action, name: 'Host version' }, revision: '3' }); await reading;
    expect(editor.state.value?.candidate.name).toBe('Typed during read');
    expect(editor.state.value?.revision).toBe('1');
    expect(editor.canAdoptReview.value).toBe(true);
    editor.change({ ...action, name: 'Edited after comparison' });
    expect(editor.adoptReview()).toBe(false);
    expect(editor.state.value?.revision).toBe('1');
  });

  test.each(['navigate', 'dispose'] as const)('a review read is fenced on %s', async (operation) => {
    const pending = deferred<AutomationRecordVersion>(); const host = gateway(); host.read = vi.fn(() => pending.promise);
    const editor = useAutomationEditor(host); editor.open('action', action, '1');
    const reading = editor.readForReview();
    if (operation === 'dispose') editor.dispose(); else editor.open('event', event, '7');
    pending.resolve({ record: action, revision: '3' }); await reading;
    expect(editor.review.value).toBe(null);
    expect(editor.reading.value).toBe(false);
    if (operation === 'navigate') expect(editor.state.value?.revision).toBe('7');
    else expect(editor.state.value).toBe(null);
  });

  test('delete failures retain the draft; deletion completed after newer edits keeps those edits for review', async () => {
    const host = gateway(); host.remove = vi.fn(async () => { throw new ControlCallError('revision_conflict', 'Host changed'); });
    const editor = useAutomationEditor(host); editor.open('action', action, '1');
    editor.change({ ...action, name: 'Keep on failure' }); await editor.remove();
    expect(editor.state.value).toMatchObject({ reviewRequired: true, candidate: { name: 'Keep on failure' } });
    editor.open('action', action, '1');
    const pending = deferred<void>(); host.remove = vi.fn(() => pending.promise);
    const deleting = editor.remove(); editor.change({ ...action, name: 'New while deleting' });
    pending.resolve(); await deleting;
    expect(editor.state.value).toMatchObject({ reviewRequired: true, candidate: { name: 'New while deleting' } });
  });

  test('missing metadata blocks submission, while explicit in-memory revisions remain distinct from missing versions', async () => {
    const host = gateway(); const editor = useAutomationEditor(host);
    editor.open('action', action, undefined); editor.change({ ...action, name: 'Cannot submit' }); await editor.save();
    expect(host.save).not.toHaveBeenCalled();
    editor.open('event', event, null, true); await editor.save();
    expect(host.save).toHaveBeenCalledWith('event', event, { revision: null, create: true });
  });
});
