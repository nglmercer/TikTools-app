import { describe, expect, test, vi } from 'vitest';
import type { LiveAction } from '../../../../automation/behavior/types.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ownerMutationGate } from '@/shared/api/owner-mutation-gate.ts';
import { useAutomationEditor } from '@/features/edit-automation-record/model/automation-editor.ts';
import { nativeDefinitionHost } from '../testing/native-definition-fixture.ts';
import { parseDefinitionState, parseDefinitionSubmission, type DefinitionState, type DefinitionWrite } from './definition-contracts.ts';
import { definitionJournal } from './definition-journal.ts';

const source = (): LiveAction => ({ schemaVersion: 2, id: 'saved', name: 'Saved action', enabled: false, typeId: 'core.log', config: { message: 'private source' } });
function fixture() {
  let record = source(); let revision = '1'; let writes = 0;
  const native = nativeDefinitionHost((method, params, options) => {
    if (['automation.create', 'automation.update', 'automation.enable', 'automation.disable'].includes(method)) {
      ++writes; revision = String(BigInt(String(options?.meta?.expectedRevision)) + 1n);
      record = structuredClone(params?.record as LiveAction ?? { ...record, enabled: method === 'automation.enable' });
      options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: 'action', id: record.id, revision }] });
      return record;
    }
    throw new Error(`unexpected ${method}`);
  });
  const transport = createFakeControlClient(native.handle);
  const journal = definitionJournal(transport.client);
  const input = { operation: 'update' as const, kind: 'action' as const, id: 'saved', expectedRevision: '1', record: source() };
  return { ...transport, native, journal, input, host: { hostId: 'host-a', hostInstanceId: 'instance-a' }, writes: () => writes, record: () => record, revision: () => revision };
}

describe('native Owner definition journal', () => {
  test('fresh acknowledgement advances only the matching editor, lost reply recovers metadata without resubmitting', async () => {
    const f = fixture();
    const editor = useAutomationEditor({ read: async () => ({ record: f.record(), revision: f.revision() }),
      save: async (_kind, record, base) => { const ack = await f.journal.submit({ ...f.input, record, expectedRevision: base.revision!, operation: base.create ? 'create' : 'update' }, f.host); return { record: ack.value as LiveAction, revision: ack.resourceVersions[0]!.revision }; },
      remove: async () => {} });
    editor.open('action', source(), '1'); editor.change({ ...source(), name: 'Submitted' });
    f.native.loseNextReply();
    await editor.save();
    expect(f.writes()).toBe(1); expect(editor.state.value?.reviewRequired).toBe(true);
    expect(editor.state.value?.revision).toBe('1');
    expect(f.journal.state.value?.recent[0]?.outcome).toBe('succeeded');
    editor.change({ ...source(), name: 'Newer unsaved text' });
    await f.journal.resolve(); editor.requireRecoveryReview();
    expect(editor.state.value?.candidate.name).toBe('Newer unsaved text');
    expect(editor.state.value?.base?.name).toBe('Saved action');
    expect(editor.state.value?.revision).toBe('1'); expect(f.writes()).toBe(1);
    await editor.save(); expect(f.writes()).toBe(1);
    await editor.readForReview(); expect(editor.adoptReview()).toBe(true);
    expect(editor.state.value?.revision).toBe('2');
    expect(editor.state.value?.candidate.name).toBe('Newer unsaved text');
    expect(f.calls.filter((c) => c.method === 'automation.definition.submit')).toHaveLength(1);
    editor.dispose();
  });

  test('a create keeps its target and tombstone revision through uncertainty and reload recovery', async () => {
    const f = fixture(); f.native.loseNextReply();
    await expect(f.journal.submit({ ...f.input, operation: 'create', expectedRevision: '9007199254740993' }, f.host)).rejects.toThrow();
    const host = f.journal.state.value!;
    expect(host.recent[0]?.intent).toMatchObject({ id: 'saved', operation: 'create', expectedRevision: '9007199254740993' });
    const reloaded = createFakeControlClient(f.native.handle);
    const recovered = definitionJournal(reloaded.client);
    await recovered.refresh(); await recovered.resolve();
    expect(recovered.state.value?.recent[0]?.receipt?.revision).toBe('9007199254740994');
    expect(f.writes()).toBe(1);
    expect(reloaded.calls.every((c) => c.method !== 'automation.definition.submit')).toBe(true);
  });

  test('pending authority transitions, shared in-flight operations and host changes block before submit', async () => {
    const f = fixture(); await f.journal.refresh();
    f.native.transitions.adoptionPending = true;
    await expect(f.journal.submit(f.input, f.host)).rejects.toMatchObject({ code: 'command_pending' });
    expect(f.writes()).toBe(0);
    f.native.transitions.adoptionPending = false;
    const gate = ownerMutationGate(f.client);
    let release!: () => void;
    const held = gate.run('approval', () => new Promise<void>((resolve) => { release = resolve; }));
    await expect(f.journal.submit(f.input, f.host)).rejects.toMatchObject({ code: 'command_pending' });
    release(); await held;
    f.native.replaceState({ ...f.native.state(), hostInstanceId: 'replacement' });
    await expect(f.journal.submit(f.input, f.host)).rejects.toMatchObject({ code: 'host_restarted' });
    expect(f.calls.some((c) => c.method === 'automation.definition.submit')).toBe(false);
  });

  test('unsafe source is refused before native requests and no journal writes browser storage', async () => {
    const f = fixture(); const storing = vi.spyOn(Storage.prototype, 'setItem');
    try {
      await expect(f.journal.submit({ ...f.input, record: { ...source(), config: { value: 9007199254740992 } } }, f.host)).rejects.toMatchObject({ code: 'unsafe_source' });
      expect(f.calls).toHaveLength(0);
      await f.journal.submit(f.input, f.host); await f.journal.resolve();
      expect(storing).not.toHaveBeenCalled();
    } finally { storing.mockRestore(); }
  });

  test('the native journal cannot lend its cached current host to an unbound definition write', async () => {
    const f = fixture(); await f.journal.refresh(); f.calls.length = 0;
    await expect(f.journal.submit(f.input, undefined as unknown as typeof f.host)).rejects.toMatchObject({ code: 'capability_unavailable' });
    expect(f.calls).toHaveLength(0); expect(f.writes()).toBe(0);
  });

  test('captures source before asynchronous preflight and refuses a second editor instead of queueing it', async () => {
    const f = fixture(); const writing = f.journal.submit(f.input, f.host);
    f.input.record.name = 'Typed later'; f.input.record.config.message = 'new draft source';
    Object.assign(f.input, { operation: 'create', id: 'other-target', expectedRevision: '9' });
    f.host.hostId = 'later-host';
    await expect(f.journal.submit({ ...f.input, expectedRevision: '1' }, f.host)).rejects.toMatchObject({ code: 'command_pending' });
    const ack = await writing;
    expect((ack.value as LiveAction).name).toBe('Saved action');
    expect((ack.value as LiveAction).config.message).toBe('private source');
    expect(f.writes()).toBe(1);
    expect(f.calls.filter((c) => c.method === 'automation.definition.submit')).toHaveLength(1);
    expect(f.calls.find((c) => c.method === 'automation.definition.submit')?.params).toMatchObject({ operation: 'update', id: 'saved', expectedRevision: '1', hostId: 'host-a' });
  });

  test('known success without a fresh acknowledgement requires review and cannot borrow an older rejection', async () => {
    const f = fixture();
    const transport = createFakeControlClient(async (method, params, options) => {
      const result = await f.native.handle(method, params, options);
      if (method !== 'automation.definition.submit') return result;
      const submission = result as { state: DefinitionState; acknowledgement: unknown };
      const success = submission.state.recent[0]!;
      submission.state.recent.push({ ...success, intent: { ...success.intent, commandId: `owner-definition-${'f'.repeat(32)}` }, outcome: 'rejected', receipt: null, errorCode: 'revision_conflict' });
      return { state: submission.state, acknowledgement: null };
    });
    const journal = definitionJournal(transport.client);
    await expect(journal.submit(f.input, f.host)).rejects.toMatchObject({ code: 'unconfirmed' });
    expect(journal.state.value?.recent[0]?.outcome).toBe('succeeded');
    expect(f.writes()).toBe(1);
    expect(transport.calls.filter((c) => c.method === 'automation.definition.submit')).toHaveLength(1);
  });

  test('recovery never archives a changed pending identity or consent invalidated during the read', async () => {
    const f = fixture();
    const pending = { commandId: `owner-definition-${'f'.repeat(32)}`, operation: 'update' as const, kind: 'action' as const, id: 'saved', expectedRevision: '1', hostInstanceId: 'instance-a', recordDigest: 'a'.repeat(64), createdAt: 1 };
    f.native.replaceState({ ...f.native.state(), pending }); await f.journal.refresh();
    await f.journal.reviewUnknown(pending.commandId, () => false);
    expect(f.native.state().pending?.commandId).toBe(pending.commandId);
    expect(f.calls.some((c) => c.method === 'automation.definition.review_unknown')).toBe(false);
    f.native.replaceState({ ...f.native.state(), pending: { ...pending, commandId: `owner-definition-${'e'.repeat(32)}` } });
    await f.journal.reviewUnknown(pending.commandId);
    expect(f.calls.some((c) => c.method === 'automation.definition.review_unknown')).toBe(false);
  });
});

test('strict definition metadata rejects added source, malformed revisions and mismatched receipts', () => {
  const state: DefinitionState = { hostId: 'host', hostInstanceId: 'instance', pending: null, recent: [] };
  expect(() => parseDefinitionState({ ...state, source: source() })).toThrow(ControlCallError);
  const intent = { commandId: `owner-definition-${'a'.repeat(32)}`, operation: 'update', kind: 'action', id: 'saved', expectedRevision: '1', hostInstanceId: 'instance', recordDigest: 'a'.repeat(64), createdAt: 1 };
  for (const pending of [{ ...intent, expectedRevision: '01' }, { ...intent, createdAt: 9007199254740992 }, { ...intent, recordDigest: null }, { ...intent, source: source() }]) {
    expect(() => parseDefinitionState({ ...state, pending })).toThrow(ControlCallError);
  }
  expect(() => parseDefinitionState({ ...state, recent: [{ intent, outcome: 'succeeded', receipt: { revision: '3', recordDigest: 'a'.repeat(64), enabled: false, deleted: null, affectedResources: 1 }, errorCode: null, recordedAt: 2 }] })).toThrow();
});

test('fresh delete acknowledgement supports complete cascades and rejects cross-target or duplicate metadata', () => {
  const input: DefinitionWrite = { operation: 'delete', kind: 'action', id: 'saved', expectedRevision: '1' };
  const intent = { commandId: `owner-definition-${'a'.repeat(32)}`, ...input, hostInstanceId: 'instance', recordDigest: null, createdAt: 1 };
  const versions = [{ kind: 'automation', scope: 'action', id: 'saved', revision: '2' }, ...Array.from({ length: 600 }, (_, i) => ({ kind: 'automation', scope: 'event', id: `event-${String(i).padStart(4, '0')}`, revision: '9' }))];
  const state = { hostId: 'host', hostInstanceId: 'instance', pending: null, recent: [{ intent, outcome: 'succeeded', receipt: { revision: '2', recordDigest: null, enabled: null, deleted: true, affectedResources: versions.length }, errorCode: null, recordedAt: 2 }] };
  const raw = { state, acknowledgement: { value: { id: 'saved', deleted: true }, resourceVersions: versions, command: { commandId: intent.commandId, state: 'succeeded', resource: { kind: 'automation', scope: 'action', id: 'saved' }, committedAt: 2 }, hostInstanceId: 'instance' } };
  expect(parseDefinitionSubmission(raw, input, state).acknowledgement?.resourceVersions).toHaveLength(601);
  expect(() => parseDefinitionSubmission({ ...raw, acknowledgement: { ...raw.acknowledgement, value: { id: 'other', deleted: true } } }, input, state)).toThrow();
  expect(() => parseDefinitionSubmission({ ...raw, acknowledgement: { ...raw.acknowledgement, resourceVersions: [versions[0], ...versions.slice(1, -1), versions[1]] } }, input, state)).toThrow();
  const zeroInput = { ...input, expectedRevision: '0' };
  const zero = { state: { ...state, recent: [{ ...state.recent[0]!, intent: { ...intent, expectedRevision: '0' },
    receipt: { revision: '0', recordDigest: null, enabled: null, deleted: false, affectedResources: 1 } }] }, acknowledgement: {
    ...raw.acknowledgement, value: { id: 'saved', deleted: false }, resourceVersions: [{ ...versions[0]!, revision: '0' }],
  } };
  expect(parseDefinitionSubmission(zero, zeroInput, state).acknowledgement?.value).toEqual({ id: 'saved', deleted: false });
});

test('fresh save acknowledgement binds the complete normalized captured body including blank Unicode names', () => {
  const host = { hostId: 'host', hostInstanceId: 'instance' };
  for (const name of ['', '\u0085', '\uFEFF', 'Explicit name']) {
    const input: DefinitionWrite = { operation: 'create', kind: 'action', id: 'saved', expectedRevision: '0', record: { ...source(), name } };
    const actual = { ...input.record, name: name === '' || name === '\u0085' ? 'Untitled action' : name };
    const intent = { commandId: `owner-definition-${'a'.repeat(32)}`, operation: input.operation, kind: input.kind, id: input.id,
      expectedRevision: input.expectedRevision, hostInstanceId: host.hostInstanceId, recordDigest: 'a'.repeat(64), createdAt: 1 };
    const state = { ...host, pending: null, recent: [{ intent, outcome: 'succeeded', receipt: { revision: '1', recordDigest: intent.recordDigest,
      enabled: false, deleted: null, affectedResources: 1 }, errorCode: null, recordedAt: 2 }] };
    const raw = { state, acknowledgement: { value: actual, resourceVersions: [{ kind: 'automation', scope: 'action', id: 'saved', revision: '1' }],
      hostInstanceId: host.hostInstanceId, command: { commandId: intent.commandId, state: 'succeeded', resource: { kind: 'automation', scope: 'action', id: 'saved' }, committedAt: 2 } } };
    expect(parseDefinitionSubmission(raw, input, host).acknowledgement?.value).toEqual(actual);
    for (const changed of [
      { ...actual, config: { message: 'altered response source' } },
      { ...actual, name: 'altered response name' },
      { ...actual, config: {} },
      Object.fromEntries(Object.entries(actual).filter(([key]) => key !== 'config')),
    ]) expect(() => parseDefinitionSubmission({ ...raw, acknowledgement: { ...raw.acknowledgement, value: changed } }, input, host)).toThrow(ControlCallError);
  }
});
