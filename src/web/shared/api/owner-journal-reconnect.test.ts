import { flushPromises } from '@vue/test-utils';
import { expect, test } from 'vitest';
import { definitionJournal } from '@/entities/automation/model/definition-journal.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { libraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { adoptionJournal } from '@/features/review-automation-proposal/model/adoption-journal.ts';
import { approvalJournal } from '@/features/review-automation-approval/model/approval-journal.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { ControlCallError } from './control-client.ts';
import { ownerMutationGate } from './owner-mutation-gate.ts';
import { recoverOwnerJournalsOnReconnect } from './owner-journal-reconnect.ts';

function fixture() {
  const native = nativeDefinitionHost(() => { throw new Error('Metadata recovery must not read or write source.'); });
  let online = true;
  let hold: Promise<never> | undefined;
  const fake = createFakeControlClient((method, params, options) => {
    if (hold && method === 'automation.definition.get') { const held = hold; hold = undefined; return held; }
    if (!online) throw new ControlCallError('transport', 'connection lost');
    return native.handle(method, params, options);
  });
  const gaps = new Set<() => void>();
  fake.client.onGap = (handler) => { const notify = () => handler(0); gaps.add(notify); return () => { gaps.delete(notify); }; };
  const definitions = definitionJournal(fake.client);
  const library = libraryJournal(fake.client);
  const adoption = adoptionJournal(fake.client);
  const approval = approvalJournal(fake.client);
  const journals = [definitions, library, adoption, approval];
  const stop = recoverOwnerJournalsOnReconnect(fake.client, journals);
  const committed = () => native.replaceState({ ...native.state(), recent: [{
    intent: { commandId: `owner-definition-${'a'.repeat(32)}`, operation: 'update', kind: 'action', id: 'original-id',
      expectedRevision: '1', hostInstanceId: 'instance-a', recordDigest: 'b'.repeat(64), createdAt: 1 },
    outcome: 'succeeded', receipt: { revision: '2', recordDigest: 'b'.repeat(64), enabled: false, deleted: null, affectedResources: 1 },
    errorCode: null, recordedAt: 2,
  }] });
  return { ...fake, definitions, library, journals, stop, gaps, committed,
    online: (value: boolean) => { online = value; },
    hold: (value: Promise<never>) => { hold = value; },
    gap: () => { for (const notify of gaps) notify(); },
  };
}

test('reconnect heals every native journal after history was opened offline, without replaying or adopting source', async () => {
  const f = fixture();
  try {
    await Promise.all(f.journals.map((journal) => journal.refresh()));
    f.committed(); f.online(false);
    await Promise.all(f.journals.map((journal) => journal.refresh()));
    expect(f.journals.every((journal) => journal.unavailable.value)).toBe(true);
    expect(f.definitions.state.value?.recent).toEqual([]);
    f.calls.length = 0; f.online(true); f.gap(); await flushPromises();
    expect(f.journals.every((journal) => !journal.unavailable.value && journal.error.value === '')).toBe(true);
    expect(f.definitions.state.value?.recent[0]?.intent.id).toBe('original-id');
    expect(f.definitions.state.value?.recent[0]?.receipt?.revision).toBe('2');
    expect(f.library.snapshot.value).toBeNull(); expect(f.library.draft.value).toBeNull();
    expect(f.calls.map((call) => call.method).sort()).toEqual([
      'automation.adoption.get', 'automation.approval.get', 'automation.definition.get', 'automation.library.get',
    ]);
  } finally { f.stop(); }
  expect(f.gaps.size).toBe(0);
  const count = f.calls.length; f.gap(); await flushPromises(); expect(f.calls).toHaveLength(count);
});

test('a reconnect overlapping a failed metadata read waits for its completion and the shared mutation gate', async () => {
  const f = fixture();
  let reject!: (error: unknown) => void;
  try {
    f.hold(new Promise<never>((_resolve, fail) => { reject = fail; }));
    const offlineRead = f.definitions.refresh();
    f.committed(); f.gap(); f.gap();
    expect(f.calls).toHaveLength(1);
    let release!: () => void;
    const active = ownerMutationGate(f.client).run('library', () => new Promise<void>((resolve) => { release = resolve; }));
    reject(new ControlCallError('transport', 'old connection lost')); await offlineRead; await flushPromises();
    expect(f.calls).toHaveLength(1);
    release(); await active; await flushPromises();
    expect(f.calls).toHaveLength(5);
    expect(f.definitions.unavailable.value).toBe(false);
    expect(f.definitions.state.value?.recent[0]?.outcome).toBe('succeeded');
    expect(f.calls.every((call) => call.method.endsWith('.get'))).toBe(true);
  } finally { f.stop(); }
});
