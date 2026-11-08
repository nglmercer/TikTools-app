import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { approvalFixture, copy, deferred, nativeId } from '../testing/fixture.ts';
import { useApprovalReview } from './approval-review.ts';
import { parseApprovalReview } from './contracts.ts';
import { parseApprovalState, type ApprovalState } from './native-contracts.ts';
import { ownerMutationGate } from '@/shared/api/owner-mutation-gate.ts';

async function reviewed() {
  const host = approvalFixture();
  const model = useApprovalReview(host.client);
  model.select({ kind: 'event', id: 'same' }); model.observe(host.projection());
  await model.readSaved();
  expect(model.error.value).toBe('');
  return { host, model };
}
describe('owner delegated approval review', () => {
  it('invalidates cached consent when a definition change starts or its native pending state changes', async () => {
    const { host, model } = await reviewed(); model.acknowledged.value = true;
    expect(model.canActivate.value).toBe(true);
    await ownerMutationGate(host.client).run('definition', async () => {
      expect(model.canActivate.value).toBe(false); expect(model.acknowledged.value).toBe(false);
    });
    await model.readSaved(); model.acknowledged.value = true;
    host.definitionState = { hostId: host.state.hostId, hostInstanceId: host.state.hostInstanceId, recent: [], pending: {
      commandId: `owner-definition-${'a'.repeat(32)}`, operation: 'update', kind: 'action', id: 'same', expectedRevision: '2',
      hostInstanceId: host.state.hostInstanceId, recordDigest: 'b'.repeat(64), createdAt: 1,
    } };
    await model.definitions.fetchState();
    expect(model.stale.value).toBe(true); expect(model.acknowledged.value).toBe(false); expect(model.canActivate.value).toBe(false);
    expect(await model.activate()).toBe(false);
    expect(host.calls.filter((c) => c.method === 'automation.approval.submit')).toHaveLength(0);
    model.dispose();
  });
  it('accepts eligible exact HTTP, audio, emit and reviewed script policies while rejecting mismatched effects', () => {
    const cases = [
      { type: 'core.fetch', kind: 'http', caps: ['http.request'], config: { url: 'https://hooks.example/live', maxResponseBytes: 1024 } },
      { type: 'core.audio.play', kind: 'audio', caps: ['audio.play'], config: { fileRef: { path: '/media/alert.wav' }, volume: 0.5, overlap: 'drop' } },
      { type: 'core.emit', kind: 'emit', caps: ['events.emit'], config: { type: 'moderator.alert', data: {} } },
      { type: 'core.code', kind: 'script', caps: ['scripts.execute', 'live.read'], config: { source: 'return { log: ["reviewed"] };' } },
    ] as const;
    for (const policy of cases) {
      const host = approvalFixture();
      Object.assign(host.source.dependencies[0]!.record!, { typeId: policy.type, config: policy.config });
      host.subjects[0]!.grant.capabilities = [...new Set([...host.subjects[0]!.grant.capabilities, ...policy.caps])];
      host.subjects[0]!.grant.effects = {
        http: [{ origin: 'https://hooks.example', methods: ['POST'], paths: ['/live'], queryFields: [], headerNames: [], renderFields: [], bodyFields: [], allowTextBody: false, maxBodyBytes: 0, maxResponseBytes: 1024, allowPrivateNetwork: false }],
        audio: [{ fileRefs: ['/media/alert.wav'], output: 'host-default', maxVolume: 0.5, overlaps: ['drop'] }], emitTopics: ['moderator.alert'],
      };
      const raw = host.review();
      Object.assign(raw.manifest!.effects[0]!, { kind: policy.kind, decision: policy.kind === 'script' ? 'review-required' : 'classified', renderFields: [], dynamicTarget: policy.kind === 'script', requiredCapabilities: [...policy.caps], reasons: policy.kind === 'script' ? ['opaque-script'] : [] });
      if (policy.kind === 'script') raw.manifest!.reasons.push('opaque-script');
      const meta = { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId };
      expect(parseApprovalReview(raw, meta, host.source).manifest?.effects[0]!.kind).toBe(policy.kind);
      const mismatched = copy(raw); mismatched.manifest!.effects[0]!.kind = 'plugin';
      expect(() => parseApprovalReview(mismatched, meta, host.source)).toThrow(ControlCallError);
      const privateField = copy(raw); privateField.manifest!.effects[0]!.renderFields = ['event.settings.secret'];
      expect(() => parseApprovalReview(privateField, meta, host.source)).toThrow(ControlCallError);
    }
  });
  it('accepts exact typed effect restrictions alongside legacy five-field grants', () => {
    const host = approvalFixture();
    host.subjects[0]!.grant.effects = { http: [], audio: [], emitTopics: ['moderator.alert'] };
    const meta = { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId };
    expect(parseApprovalReview(host.review(), meta, host.source).subjects[0]!.grant.effects?.emitTopics).toEqual(['moderator.alert']);
    const invalid = copy(host.review());
    Object.assign(invalid.subjects[0]!.grant.effects!, { unrestricted: true });
    expect(() => parseApprovalReview(invalid, meta, host.source)).toThrow(ControlCallError);
    host.subjects[0]!.grant.effects = { http: [], audio: [], emitTopics: [] };
    expect(() => parseApprovalReview(host.review(), meta, host.source)).toThrow(ControlCallError);
  });
  it('projects exact target-only review parameters during initial read and activation preflight against a strict host', async () => {
    const host = approvalFixture();
    const model = useApprovalReview(host.client);
    model.select(host.source);
    expect(model.target.value).toEqual({ kind: 'event', id: 'same' });
    await model.readSaved();
    model.acknowledged.value = true;
    expect(await model.activate()).toBe(true);
    const reads = host.requests.filter((r) => r.method === 'automation.approval.review');
    expect(reads).toHaveLength(2);
    expect(reads.map((r) => r.params)).toEqual([{ kind: 'event', id: 'same' }, { kind: 'event', id: 'same' }]);
    expect(() => host.handle('automation.approval.review', { ...host.source })).toThrow(ControlCallError);
  });

  it('submits only the exact reviewed identity and explicit delegated state plan through native ownership', async () => {
    const { host, model } = await reviewed();
    expect(model.review.value?.source.dependencies).toHaveLength(3);
    model.acknowledged.value = true;
    model.includeDependency('owner', true);
    expect(model.enablePlan.value).toEqual({});
    model.includeDependency('disabled', true);
    expect(model.acknowledged.value).toBe(false);
    model.setDependencyEnabled('disabled', false);
    model.includeDependency('same', true);
    model.acknowledged.value = true;
    expect(await model.activate()).toBe(true);
    expect(host.requests.filter((r) => r.method === 'automation.approval.submit')).toHaveLength(1);
    const sent = host.requests.find((r) => r.method === 'automation.approval.submit')!;
    expect(sent.params).toEqual({ operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostId: 'host-a', hostInstanceId: 'instance-a',
      reviewedVersions: host.versions, manifestDigest: 'd'.repeat(64), enablePlan: [{ kind: 'event', id: 'same', enabled: true },
        { kind: 'action', id: 'same', enabled: true }, { kind: 'action', id: 'disabled', enabled: false }], acknowledgeEffects: true });
    expect(sent.options?.meta).toBeUndefined();
    expect(sent.params).not.toHaveProperty('commandId');
    expect(sent.params).not.toHaveProperty('source');
    expect(host.requests.some((r) => ['automation.approval.activate', 'automation.approval.revoke', 'automation.enable', 'automation.update'].includes(r.method))).toBe(false);
    expect(model.journal.state.value?.recent[0]?.intent.commandId).toBe(nativeId());
    expect(model.stale.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
    expect(model.enablePlan.value).toEqual({ disabled: false, same: true });
  });

  it('leaves omitted dependencies unchanged and never treats a preview as authorization', async () => {
    const { host, model } = await reviewed();
    expect(model.review.value?.manifest?.executionAuthorized).toBe(false);
    expect(await model.activate()).toBe(false);
    model.acknowledged.value = true;
    await model.activate();
    expect(host.requests.find((r) => r.method === 'automation.approval.submit')?.params?.enablePlan)
      .toEqual([{ kind: 'event', id: 'same', enabled: true }]);
  });

  it('preserves explicit state plans for canonical IDs that match JavaScript prototype keys', async () => {
    for (const id of ['__proto__', 'constructor', 'toString']) {
      const host = approvalFixture();
      host.source.dependencies[1]!.id = id; host.source.dependencies[1]!.record!.id = id;
      (host.source.record!.actionIds as string[])[1] = id; host.versions[2]!.id = id;
      host.subjects[0]!.grant.resources.automation![2] = `action:${id}`;
      const model = useApprovalReview(host.client);
      model.select({ kind: 'event', id: 'same' }); await model.readSaved();
      model.includeDependency(id, true);
      expect(Object.hasOwn(model.enablePlan.value, id)).toBe(true);
      model.setDependencyEnabled(id, false); model.acknowledged.value = true;
      expect(await model.activate()).toBe(true);
      expect(host.requests.find((r) => r.method === 'automation.approval.submit')?.params?.enablePlan)
        .toEqual([{ kind: 'event', id: 'same', enabled: true }, { kind: 'action', id, enabled: false }]);
      model.dispose();
    }
  });

  it('keeps captured source and plan on stale preflight and requires fresh explicit review and consent', async () => {
    const { host, model } = await reviewed();
    model.includeDependency('disabled', true); model.acknowledged.value = true;
    const old = model.review.value;
    host.source.dependencies[1]!.record!.name = 'New disabled alternative'; host.versions[2]!.revision = '4';
    expect(await model.activate()).toBe(false);
    expect(model.review.value).toBe(old);
    expect(model.enablePlan.value).toEqual({ disabled: true });
    expect(model.notice.value).toBe('changed');
    expect(model.acknowledged.value).toBe(false);
    expect(host.requests.some((r) => r.method === 'automation.approval.submit')).toBe(false);
    await model.readSaved();
    expect(model.review.value?.source.dependencies[1]!.record!.name).toBe('New disabled alternative');
    expect(model.stale.value).toBe(false);
    expect(model.acknowledged.value).toBe(false);
    model.acknowledged.value = true;
    expect(await model.activate()).toBe(true);
  });

  it('invalidates consent for permission/configuration changes even when definition revisions remain the same', async () => {
    const { host, model } = await reviewed();
    model.acknowledged.value = true;
    host.subjects[0]!.configurationRevision = '3';
    expect(await model.activate()).toBe(false);
    expect(model.notice.value).toBe('changed');
    expect(host.requests.some((r) => r.method === 'automation.approval.submit')).toBe(false);
  });

  it('retains source and plan but removes consent after an unverified preflight response', async () => {
    const { host, model } = await reviewed();
    model.includeDependency('disabled', true); model.acknowledged.value = true;
    const captured = model.review.value;
    host.overrides.set('automation.approval.review', () => host.review());
    expect(await model.activate()).toBe(false);
    expect(model.stale.value).toBe(true);
    expect(model.acknowledged.value).toBe(false);
    expect(model.review.value).toBe(captured);
    expect(model.enablePlan.value).toEqual({ disabled: true });
    expect(host.requests.some((r) => r.method === 'automation.approval.submit')).toBe(false);
  });

  it('revokes retained approval with unavailable authors and a deleted primary without sending an activation proof', async () => {
    const { host, model } = await reviewed();
    host.source.record = null; host.source.dependencies = []; host.versions.splice(1); host.versions[0]!.revision = '7';
    host.manifestAvailable = false; host.eligibility = { eligible: false, reasonCode: 'definition-missing' };
    host.approval = { state: 'active', generation: '3', policyVersion: 1, manifestDigest: 'b'.repeat(64), current: false };
    await model.readSaved();
    expect(model.activateEligible.value).toBe(false);
    expect(model.revokeEligible.value).toBe(true);
    model.revokeAcknowledged.value = true;
    expect(await model.revoke()).toBe(true);
    const params = host.requests.find((r) => r.method === 'automation.approval.submit')!.params;
    expect(params).toEqual({ operation: 'revoke', kind: 'event', id: 'same', expectedRevision: '7', hostId: 'host-a', hostInstanceId: 'instance-a', acknowledgeRevocation: true });
  });

  it('blocks approval when saved source integers cannot be represented exactly while keeping retained revocation available', async () => {
    const host = approvalFixture();
    host.source.record!.filters = JSON.parse('[{"field":"event.data.count","operator":"eq","value":9007199254740993}]') as unknown[];
    host.source.dependencies[1]!.record!.unused = JSON.parse('{"large":9223372036854775807}') as unknown;
    host.approval = { state: 'active', generation: '2', policyVersion: 1, manifestDigest: 'b'.repeat(64), current: false };
    const model = useApprovalReview(host.client);
    model.select({ kind: 'event', id: 'same' }); await model.readSaved();
    expect(model.review.value?.eligibility.eligible).toBe(true);
    expect(model.review.value?.source.record).not.toBeNull();
    expect(model.sourcePrecisionSafe.value).toBe(false);
    model.acknowledged.value = true;
    expect(model.canActivate.value).toBe(false);
    expect(await model.activate()).toBe(false);
    expect(model.revokeEligible.value).toBe(true);
    model.revokeAcknowledged.value = true;
    expect(await model.revoke()).toBe(true);
    expect(host.requests.filter((r) => r.method === 'automation.approval.submit').map((r) => r.params?.operation)).toEqual(['revoke']);
  });

  it('keeps a timed-out native pending identity across a new WebView without resubmitting source or inventing another ID', async () => {
    const { host, model } = await reviewed();
    host.overrides.set('automation.approval.submit', () => {
      host.state.pending = { commandId: nativeId(), operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
      throw new ControlCallError('timeout', 'lost after native intent');
    });
    model.acknowledged.value = true;
    expect(await model.activate()).toBe(false);
    expect(model.journal.state.value?.pending?.commandId).toBe(nativeId());
    expect(model.canActivate.value).toBe(false);
    model.dispose();
    const reloaded = useApprovalReview(createFakeControlClient(host.handler).client);
    await reloaded.refreshJournals();
    expect(reloaded.journal.state.value?.pending?.commandId).toBe(nativeId());
    await reloaded.resolveOriginal();
    expect(host.requests.filter((r) => r.method === 'automation.approval.submit')).toHaveLength(1);
    expect(host.requests.find((r) => r.method === 'automation.approval.resolve')?.params).toEqual({});
    await reloaded.reviewUnknown(nativeId());
    expect(host.requests.some((r) => r.method === 'automation.approval.review_unknown')).toBe(false);
    reloaded.unknownAcknowledged.value = true;
    await reloaded.reviewUnknown(nativeId('2'));
    expect(host.requests.some((r) => r.method === 'automation.approval.review_unknown')).toBe(false);
    await reloaded.reviewUnknown(nativeId());
    expect(host.requests.find((r) => r.method === 'automation.approval.review_unknown')?.params).toEqual({ commandId: nativeId(), acknowledgeUnknown: true });
    expect(reloaded.journal.state.value?.recent[0]?.outcome).toBe('reviewed-unknown');
  });

  it('shares the pending gate with owner adoption and blocks second windows during preflight', async () => {
    const { host, model } = await reviewed();
    host.adoptionState.pending = { kind: 'event', id: 'same', commandId: 'owner-adopt-pending', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    await model.refreshJournals();
    model.acknowledged.value = true;
    expect(model.activateEligible.value).toBe(false);
    expect(await model.activate()).toBe(false);
    host.adoptionState.pending = null;
    await model.refreshJournals();
    await model.readSaved(); model.acknowledged.value = true;
    const other = useApprovalReview(host.client);
    other.select({ kind: 'event', id: 'same' }); await other.readSaved(); other.acknowledged.value = true;
    const blocked = deferred<unknown>();
    host.overrides.set('automation.approval.review', (_p, options) => { options?.onMetadata?.({ resourceVersions: copy(host.versions), hostInstanceId: host.state.hostInstanceId }); return blocked.promise; });
    const first = model.activate(); await flushPromises();
    expect(other.journal).toBe(model.journal);
    expect(await other.activate()).toBe(false);
    model.acknowledged.value = false;
    blocked.resolve(host.review());
    expect(await first).toBe(false);
    expect(host.requests.some((r) => r.method === 'automation.approval.submit')).toBe(false);
  });

  it('does not dispatch after navigation, consent removal or plan changes during delayed review', async () => {
    for (const change of ['close', 'consent', 'plan'] as const) {
      const { host, model } = await reviewed();
      const blocked = deferred<unknown>();
      host.overrides.set('automation.approval.review', (_p, options) => { options?.onMetadata?.({ resourceVersions: copy(host.versions), hostInstanceId: host.state.hostInstanceId }); return blocked.promise; });
      model.acknowledged.value = true;
      const attempt = model.activate(); await flushPromises();
      if (change === 'close') model.closeReview();
      else if (change === 'consent') model.acknowledged.value = false;
      else model.includeDependency('disabled', true);
      blocked.resolve(host.review());
      expect(await attempt).toBe(false);
      expect(host.requests.some((r) => r.method === 'automation.approval.submit')).toBe(false);
    }
  });

  it('does not archive uncertainty after its consent is hidden or removed during the final original-ID lookup', async () => {
    const host = approvalFixture();
    host.state.pending = { commandId: nativeId(), operation: 'revoke', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    const model = useApprovalReview(host.client); await model.refreshJournals();
    const blocked = deferred<ApprovalState>();
    host.overrides.set('automation.approval.get', () => blocked.promise);
    model.unknownAcknowledged.value = true;
    const archive = model.reviewUnknown(nativeId()); await flushPromises();
    model.closeReview(); blocked.resolve(copy(host.state)); await archive;
    expect(host.requests.some((r) => r.method === 'automation.approval.review_unknown')).toBe(false);
  });

  it('requires a new saved review after pending authority transitions resolve or uncertainty is archived', async () => {
    for (const operation of ['resolve', 'archive']) {
      const { host, model } = await reviewed();
      model.acknowledged.value = true;
      host.state.pending = { commandId: nativeId(), operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
      await model.journal.refresh();
      expect(model.acknowledged.value).toBe(false);
      expect(model.stale.value).toBe(true);
      if (operation === 'archive') {
        model.unknownAcknowledged.value = true;
        await model.reviewUnknown(nativeId());
      } else {
        host.overrides.set('automation.approval.resolve', () => { host.state.pending = null; return copy(host.state); });
        await model.resolveOriginal();
      }
      expect(model.journal.state.value?.pending).toBeNull();
      expect(model.activateEligible.value).toBe(false);
      expect(await model.activate()).toBe(false);
      await model.readSaved();
      expect(model.activateEligible.value).toBe(true);
      expect(model.acknowledged.value).toBe(false);
      model.dispose();
    }
  });
});

describe('closed approval wire contracts', () => {
  it('rejects missing config pins, mismatched subject grants, incomplete dependencies and unexpected authority fields', () => {
    const host = approvalFixture();
    const valid = host.review();
    expect(parseApprovalReview(valid, { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId }, host.source).subjects[0]!.configurationRevision).toBe('2');
    const mutations = [
      (v: typeof valid) => { delete (v.manifest!.subjects[0] as Partial<ExecutionManifestSubject>).configurationRevision; },
      (v: typeof valid) => { v.subjects[0]!.grantRevision = '4'; },
      (v: typeof valid) => { v.manifest!.dependencies.pop(); },
      (v: typeof valid) => { Object.assign(v.source.dependencies[0]!, { authority: 'owner' }); },
      (v: typeof valid) => { v.manifest!.executionAuthorized = true as false; },
    ];
    for (const mutate of mutations) { const broken = copy(valid); mutate(broken); expect(() => parseApprovalReview(broken, { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId }, host.source)).toThrow(ControlCallError); }
  });

  it('rejects browser-source fields, malformed result transitions and ambiguous native terminal records', () => {
    const host = approvalFixture();
    const intent = { commandId: nativeId(), operation: 'activate' as const, kind: 'event' as const, id: 'same', expectedRevision: '4', hostInstanceId: 'old-instance', createdAt: 1 };
    const record = { intent, outcome: 'succeeded' as const, receipt: { revision: '5', approvalGeneration: '1', policyVersion: 1, manifestDigest: 'a'.repeat(64) }, errorCode: null, recordedAt: 2 };
    const state: ApprovalState = { ...host.state, recent: [record] };
    expect(parseApprovalState(state).recent[0]?.receipt?.revision).toBe('5');
    for (const broken of [{ ...state, source: {} }, { ...state, recent: [{ ...record, receipt: { ...record.receipt, revision: '4' } }] },
      { ...state, recent: [{ ...record, errorCode: 'internal' }] }, { ...state, recent: [{ ...record, outcome: 'rejected', receipt: null, errorCode: 'overloaded' }] },
      { ...state, pending: intent }]) expect(() => parseApprovalState(broken)).toThrow(ControlCallError);
  });

  it('rejects coercible array enums, nonpositive intent revisions and a review without its host-instance fence', () => {
    const host = approvalFixture();
    const intent = { commandId: nativeId(), operation: 'activate', kind: 'event', id: 'same', expectedRevision: '4', hostInstanceId: 'instance-a', createdAt: 1 };
    for (const broken of [{ ...intent, operation: ['activate'] }, { ...intent, kind: ['event'] }, { ...intent, expectedRevision: '0' }]) {
      expect(() => parseApprovalState({ ...host.state, pending: broken })).toThrow(ControlCallError);
    }
    expect(() => parseApprovalState({ ...host.state, recent: [{ intent, outcome: ['reviewed-unknown'], receipt: null, errorCode: null, recordedAt: 2 }] })).toThrow(ControlCallError);
    const review = host.review();
    expect(() => parseApprovalReview(review, { resourceVersions: host.versions }, host.source)).toThrow(ControlCallError);
    const meta = { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId };
    const effectArray = copy(review); Object.assign(effectArray.manifest!.effects[0]!, { kind: ['log'] });
    expect(() => parseApprovalReview(effectArray, meta, host.source)).toThrow(ControlCallError);
    const summaryArray = copy(review); Object.assign(summaryArray.approval, { state: ['none'] });
    expect(() => parseApprovalReview(summaryArray, meta, host.source)).toThrow(ControlCallError);
    const provenanceExtra = copy(review); Object.assign(provenanceExtra.source.provenance!, { unreviewed: true });
    Object.assign(provenanceExtra.manifest!.primary.provenance!, { unreviewed: true });
    expect(() => parseApprovalReview(provenanceExtra, meta, host.source)).toThrow(ControlCallError);
  });

  it('preserves allowed grant map keys that coincide with JavaScript prototype properties', () => {
    const host = approvalFixture();
    host.subjects[0]!.grant.actions = Object.fromEntries([['__proto__', ['action.one']], ['constructor', ['action.two']], ['toString', ['action.three']]]);
    host.subjects[0]!.grant.settingsFields = Object.fromEntries([['__proto__', ['public.one']], ['constructor', []], ['toString', ['public.three']]]);
    const parsed = parseApprovalReview(host.review(), { resourceVersions: host.versions, hostInstanceId: host.state.hostInstanceId }, host.source);
    for (const map of [parsed.subjects[0]!.grant.actions, parsed.subjects[0]!.grant.settingsFields]) {
      expect(Object.keys(map)).toEqual(['__proto__', 'constructor', 'toString']);
      expect(Object.hasOwn(map, '__proto__')).toBe(true);
    }
    expect(parsed.subjects[0]!.grant.actions['__proto__']).toEqual(['action.one']);
  });
});
type ExecutionManifestSubject = { principalId: string; sessionId: string; grantRevision: string; configurationRevision: string };
