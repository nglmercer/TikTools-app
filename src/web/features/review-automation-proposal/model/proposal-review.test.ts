import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { copy, deferred, proposalFixture } from '../testing/fixture.ts';
import { parseReview } from './contracts.ts';
import { useProposalReview } from './proposal-review.ts';

async function reviewed() {
  const host = proposalFixture();
  const model = useProposalReview(host.client);
  model.select({ kind: 'event', id: 'same' });
  model.observe(host.projection());
  await model.readSaved();
  return { host, model };
}

describe('owner proposal review and native adoption recovery', () => {
  it('projects target-only source reads even when selection came from a structurally wider saved review', async () => {
    const host = proposalFixture();
    const model = useProposalReview(host.client);
    model.select(host.source);
    expect(model.target.value).toEqual({ kind: 'event', id: 'same' });
    await model.readSaved();
    model.acknowledged.value = true;
    expect(await model.adopt()).toBe(true);
    expect(host.requests.filter((r) => r.method === 'automation.review').map((r) => r.params)).toEqual([{ kind: 'event', id: 'same' }, { kind: 'event', id: 'same' }]);
    expect(() => host.handle('automation.review', { ...host.source })).toThrow(ControlCallError);
  });

  it('keeps unsafe integer source inspectable but cannot consent to a rounded representation before owner transfer', async () => {
    const host = proposalFixture();
    host.source.record!.filters = JSON.parse('[{"field":"event.data.count","value":9007199254740993}]') as unknown[];
    host.source.dependencies[0]!.record!.unused = JSON.parse('{"large":9223372036854775807}') as unknown;
    const model = useProposalReview(host.client);
    model.select({ kind: 'event', id: 'same' }); await model.readSaved();
    expect(model.review.value?.source.record).not.toBeNull();
    expect(model.sourcePrecisionSafe.value).toBe(false);
    expect(model.adoptEligible.value).toBe(false);
    model.acknowledged.value = true;
    expect(await model.adopt()).toBe(false);
    expect(host.calls.some((c) => c.method === 'automation.adoption.submit')).toBe(false);
  });

  it('blocks adoption while delegated approval is pending and requires fresh review after that transition is resolved', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    host.approvalState.pending = { commandId: `owner-approval-${'1'.repeat(32)}`, operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    await model.approvals.refresh();
    expect(model.canAdopt.value).toBe(false);
    expect(model.acknowledged.value).toBe(false);
    expect(await model.adopt()).toBe(false);
    host.approvalState.pending = null;
    await model.approvals.refresh();
    expect(model.adoptEligible.value).toBe(false);
    await model.readSaved();
    expect(model.adoptEligible.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
    model.acknowledged.value = true;
    expect(await model.adopt()).toBe(true);
  });

  it('rechecks the approval journal before adoption dispatch and shares the same client lock order', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    host.approvalState.pending = { commandId: `owner-approval-${'1'.repeat(32)}`, operation: 'revoke', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    expect(model.canAdopt.value).toBe(true);
    expect(await model.adopt()).toBe(false);
    expect(host.requests.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
    expect(model.stale.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
  });

  it('submits only an acknowledged exact saved review, host identity and every family-scoped dependency fence', async () => {
    const { host, model } = await reviewed();
    expect(model.error.value).toBe('');
    expect(model.review.value?.source.dependencies.map((d) => d.id)).toEqual(['same', 'disabled', 'missing']);
    expect(model.canAdopt.value).toBe(false);
    expect(await model.adopt()).toBe(false);
    model.acknowledged.value = true;
    expect(model.canAdopt.value).toBe(true);
    expect(await model.adopt()).toBe(true);
    const writes = host.requests.filter((r) => r.method === 'automation.adoption.submit');
    expect(writes).toHaveLength(1);
    expect(writes[0]!.params).toEqual({ kind: 'event', id: 'same', expectedRevision: '4', acknowledgeAuthority: true,
      hostId: 'host-a', hostInstanceId: 'instance-a', reviewedVersions: host.versions });
    expect(writes[0]!.options?.meta).toBeUndefined();
    expect(host.requests.filter((r) => r.method === 'automation.review')).toHaveLength(2);
    expect(host.requests.some((r) => ['automation.adopt', 'automation.enable', 'automation.update', 'automation.create'].includes(r.method))).toBe(false);
    expect(model.journal.state.value?.recent[0]?.intent.commandId).toBe('native-adoption-1');
    expect(model.canAdopt.value).toBe(false);
    expect(model.acknowledged.value).toBe(false);
  });

  it('invalidates acknowledgement when an unselected or disabled dependency revision changes', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    const current = host.projection();
    current.set('action:disabled', '4');
    model.observe(current);
    expect(model.notice.value).toBe('changed');
    expect(model.acknowledged.value).toBe(false);
    expect(await model.adopt()).toBe(false);
    expect(host.calls.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
  });

  it('lets a newer atomic review supersede a delayed older owner projection without adopting unseen changes', async () => {
    const host = proposalFixture();
    const older = host.projection();
    host.versions[2]!.revision = '4';
    host.source.dependencies[1]!.record!.name = 'Changed disabled branch';
    const model = useProposalReview(host.client);
    model.select({ kind: 'event', id: 'same' }); model.observe(older);
    await model.readSaved();
    expect(model.review.value?.source.dependencies[1]!.record!.name).toBe('Changed disabled branch');
    expect(model.stale.value).toBe(false);
    expect(model.acknowledged.value).toBe(false);
    expect(host.calls.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
    model.acknowledged.value = true;
    model.observe(new Map(older));
    expect(model.canAdopt.value).toBe(true);
    expect(await model.adopt()).toBe(true);
    expect(host.calls.find((r) => r.method === 'automation.adoption.submit')?.params?.reviewedVersions).toEqual(host.versions);
  });

  it('detects disappearance only after the projection catches up to that reviewed definition', async () => {
    const host = proposalFixture();
    const missing = host.projection(); missing.delete('action:same');
    const model = useProposalReview(host.client);
    model.select({ kind: 'event', id: 'same' }); model.observe(missing);
    await model.readSaved();
    expect(model.stale.value).toBe(false);
    model.acknowledged.value = true;
    const older = new Map(missing); older.set('action:same', '1');
    model.observe(older); model.observe(new Map(missing));
    expect(model.canAdopt.value).toBe(true);
    model.observe(host.projection());
    expect(model.canAdopt.value).toBe(true);
    model.observe(new Map(missing));
    expect(model.stale.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
    expect(model.notice.value).toBe('changed');
  });

  it('resets disappearance tracking on explicit re-review while still rejecting later newer revisions', async () => {
    const { host, model } = await reviewed();
    const missing = host.projection(); missing.delete('action:same');
    model.observe(missing);
    expect(model.stale.value).toBe(true);
    host.versions[1]!.revision = '3';
    host.source.dependencies[0]!.record!.name = 'Reviewed recreated action';
    await model.readSaved();
    expect(model.stale.value).toBe(false);
    expect(model.acknowledged.value).toBe(false);
    model.acknowledged.value = true;
    model.observe(new Map(missing));
    expect(model.canAdopt.value).toBe(true);
    const newer = host.projection(); newer.set('action:same', '4');
    model.observe(newer);
    expect(model.stale.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
  });

  it('re-reads full saved source before submission even before a newer projection arrives', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    host.source.dependencies[0]!.record!.config = { source: 'changed after review' };
    host.versions[1]!.revision = '3';
    expect(await model.adopt()).toBe(false);
    expect(model.notice.value).toBe('changed');
    expect(model.review.value?.source.dependencies[0]!.record!.config).toEqual({ source: '<script>privateSource()</script>' });
    expect(host.calls.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
  });

  it('refuses a restarted or different host captured after the review', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    host.state.hostInstanceId = 'instance-b';
    expect(await model.adopt()).toBe(false);
    expect(model.notice.value).toBe('hostChanged');
    expect(host.calls.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
  });

  it('drops a late saved review after selection changes or the component is disposed', async () => {
    for (const dispose of [false, true]) {
      const host = proposalFixture();
      const pending = deferred<unknown>();
      host.overrides.set('automation.review', (_params, options) => {
        options?.onMetadata?.({ resourceVersions: copy(host.versions) });
        return pending.promise;
      });
      const model = useProposalReview(host.client);
      model.select({ kind: 'event', id: 'same' }); model.observe(host.projection());
      const reading = model.readSaved();
      await flushPromises();
      if (dispose) model.dispose(); else model.select({ kind: 'event', id: 'other' });
      pending.resolve(copy(host.source));
      await reading;
      expect(model.review.value).toBeNull();
      expect(model.canAdopt.value).toBe(false);
    }
  });

  it('shares one pending gate, reloads native state after timeout and queries the original ID without resend', async () => {
    const { host, model } = await reviewed();
    host.overrides.set('automation.adoption.submit', () => {
      host.state.pending = { kind: 'event', id: 'same', commandId: 'persisted-on-host', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
      throw new ControlCallError('timeout', 'Acknowledgement lost');
    });
    model.acknowledged.value = true;
    expect(await model.adopt()).toBe(false);
    expect(model.notice.value).toBe('unconfirmed');
    expect(model.journal.state.value?.pending?.commandId).toBe('persisted-on-host');
    expect(await model.adopt()).toBe(false);
    model.dispose();
    // A new WebView/control instance must recover from host persistence, with
    // no localStorage/IndexedDB entry and no new submission.
    const reloadedClient = createFakeControlClient(host.handler).client;
    const reloaded = useProposalReview(reloadedClient);
    await reloaded.journal.refresh();
    expect(reloaded.journal.state.value?.pending?.commandId).toBe('persisted-on-host');
    await reloaded.journal.resolve();
    expect(host.requests.filter((r) => r.method === 'automation.adoption.submit')).toHaveLength(1);
    expect(host.requests.find((r) => r.method === 'automation.adoption.resolve')?.params).toEqual({});
    await reloaded.reviewUnknown('different-command');
    expect(host.requests.some((r) => r.method === 'automation.adoption.review_unknown')).toBe(false);
    await reloaded.reviewUnknown('persisted-on-host');
    expect(host.requests.find((r) => r.method === 'automation.adoption.review_unknown')?.params)
      .toEqual({ commandId: 'persisted-on-host', acknowledgeUnknown: true });
    expect(reloaded.journal.state.value?.pending).toBeNull();
    expect(reloaded.journal.state.value?.recent[0]?.outcome).toBe('reviewed-unknown');
  });

  it('reserves the shared journal gate throughout preflight and cancels submission after acknowledgement is removed', async () => {
    const { host, model } = await reviewed();
    const second = useProposalReview(host.client);
    second.select({ kind: 'event', id: 'same' }); second.observe(host.projection());
    await second.readSaved();
    model.acknowledged.value = true; second.acknowledged.value = true;
    const pending = deferred<unknown>();
    host.overrides.set('automation.review', (_params, options) => {
      options?.onMetadata?.({ resourceVersions: copy(host.versions) }); return pending.promise;
    });
    const firstAttempt = model.adopt();
    await flushPromises();
    expect(second.journal).toBe(model.journal);
    expect(second.canAdopt.value).toBe(false);
    expect(await second.adopt()).toBe(false);
    model.acknowledged.value = false;
    pending.resolve(copy(host.source));
    expect(await firstAttempt).toBe(false);
    expect(host.requests.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
  });

  it('never dispatches after navigation during a delayed preflight source read', async () => {
    const { host, model } = await reviewed();
    const pending = deferred<unknown>();
    host.overrides.set('automation.review', (_params, options) => {
      options?.onMetadata?.({ resourceVersions: copy(host.versions) }); return pending.promise;
    });
    model.acknowledged.value = true;
    const result = model.adopt();
    await flushPromises();
    model.select(null);
    pending.resolve(copy(host.source));
    expect(await result).toBe(false);
    expect(host.requests.some((r) => r.method === 'automation.adoption.submit')).toBe(false);
  });

  it('does not archive hidden uncertainty consent after closing during its final status read', async () => {
    const host = proposalFixture();
    host.state.pending = { kind: 'event', id: 'same', commandId: 'pending-before-close', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    const model = useProposalReview(host.client);
    await model.journal.refresh();
    const pending = deferred<unknown>();
    host.overrides.set('automation.adoption.get', () => pending.promise);
    const result = model.reviewUnknown('pending-before-close');
    await flushPromises();
    model.closeReview();
    pending.resolve(copy(host.state));
    await result;
    expect(host.requests.some((r) => r.method === 'automation.adoption.review_unknown')).toBe(false);
  });

  it('rejects missing, duplicated or unrelated dependency metadata rather than adopting a partial source', () => {
    const host = proposalFixture();
    for (const versions of [host.versions.slice(0, -1), [...host.versions, host.versions[0]!], [...host.versions].reverse(),
      host.versions.map((v, i) => i === 0 ? { ...v, privateExtra: true } : v),
      [...host.versions, { kind: 'automation', scope: 'action', id: 'unrelated', revision: '1' }]]) {
      expect(() => parseReview(host.source, { resourceVersions: versions }, { kind: 'event', id: 'same' })).toThrow(ControlCallError);
    }
    const partial = copy(host.source);
    partial.dependencies.pop();
    expect(() => parseReview(partial, { resourceVersions: host.versions }, { kind: 'event', id: 'same' })).toThrow(ControlCallError);
    const reordered = copy(host.source);
    reordered.dependencies.reverse();
    const reorderedVersions = [host.versions[0]!, ...host.versions.slice(1).reverse()];
    expect(() => parseReview(reordered, { resourceVersions: reorderedVersions }, { kind: 'event', id: 'same' })).toThrow(ControlCallError);
  });

  it('keeps incomplete legacy source inspectable without pretending to validate or repair its executable schema', () => {
    const host = proposalFixture();
    delete host.source.record!.trigger;
    delete host.source.dependencies[0]!.record!.typeId;
    const parsed = parseReview(host.source, { resourceVersions: host.versions }, { kind: 'event', id: 'same' });
    expect(parsed.source.record).not.toHaveProperty('trigger');
    expect(parsed.source.dependencies[0]!.record).not.toHaveProperty('typeId');
    expect(parsed.source.provenance?.authority.kind).toBe('delegated');
  });

  it('binds duplicate action references once in their original first-occurrence order', () => {
    const host = proposalFixture();
    host.source.record!.actionIds = ['same', 'same', 'disabled', 'same', 'missing'];
    const parsed = parseReview(host.source, { resourceVersions: host.versions }, { kind: 'event', id: 'same' });
    expect(parsed.source.dependencies.map((d) => d.id)).toEqual(['same', 'disabled', 'missing']);
    expect(parsed.versions).toEqual(host.versions);
  });
});
