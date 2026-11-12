import type { Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { callWithMeta, ControlCallError } from '@/shared/api/control-client.ts';
import type { DefinitionJournal } from './definition-journal.ts';
import { assertRecord, recordRevision, validRevision,
  type AutomationHost, type AutomationKind, type AutomationRecord, type AutomationRecordVersion,
  type AutomationRevision, type AutomationWriteBase,
} from './record-versions.ts';

interface RecordGatewayContext {
  persistent: Readonly<Ref<boolean | null>>;
  definitions: Pick<DefinitionJournal, 'fetchState' | 'submit'>;
  isDisposed: () => boolean;
  invalidateRefresh: () => void;
  refresh: () => Promise<void>;
}

/** Checked record protocol; the root retains journal, error and refresh ownership. */
export function createAutomationRecordGateway(control: ControlClient, context: RecordGatewayContext) {
  const { persistent, definitions } = context;
  const checkedOptions = (revision: AutomationRevision) => {
    if (context.isDisposed()) throw new ControlCallError('detached', 'Automation state was disposed.');
    if (revision === null) {
      if (persistent.value !== false) throw new ControlCallError('capability_unavailable', 'Read the current record revision before writing.');
      return undefined;
    }
    if (!validRevision(revision)) throw new ControlCallError('invalid_params', 'Invalid automation base revision.');
    return { meta: { expectedRevision: revision } };
  };
  const checkedReadHost = (before: AutomationHost | null, after: AutomationHost | null): AutomationHost | null => {
    if (before && after && (before.hostId !== after.hostId || before.hostInstanceId !== after.hostInstanceId)) {
      throw new ControlCallError('host_restarted', 'The Owner host changed while reading source. Read again.');
    }
    return before && after ? { hostId: before.hostId, hostInstanceId: before.hostInstanceId } : null;
  };
  const checkedWriteHost = (host: AutomationHost | null | undefined): AutomationHost => {
    if (!host) throw new ControlCallError('capability_unavailable', 'Read and review the Owner host identity before writing.');
    return host;
  };
  const readRecord = async (kind: AutomationKind, id: string): Promise<AutomationRecordVersion> => {
    if (context.isDisposed()) throw new ControlCallError('detached', 'Automation state was disposed.');
    const before = persistent.value === false ? null : await definitions.fetchState().catch(() => null);
    const readHost = async () => checkedReadHost(before, persistent.value === false ? null : await definitions.fetchState().catch(() => null));
    try {
      const response = await callWithMeta<AutomationRecord>(control, 'automation.get', { id, kind });
      assertRecord(response.result, kind, id);
      const host = await readHost();
      return { record: response.result, revision: recordRevision(response.meta, kind, id, persistent.value === false), ...(persistent.value !== false ? { host } : {}) };
    } catch (failure) {
      if (!(failure instanceof ControlCallError) || failure.code !== 'automation_not_found') throw failure;
      const data = failure.data as { resourceVersions?: unknown } | undefined;
      const meta = Array.isArray(data?.resourceVersions) ? { resourceVersions: data.resourceVersions } : undefined;
      const host = await readHost();
      return { record: null, revision: recordRevision(meta, kind, id, persistent.value === false), ...(persistent.value !== false ? { host } : {}) };
    }
  };
  const mutateVersioned = async <T>(work: () => Promise<T>): Promise<T> => {
    // An older snapshot read cannot rewind a successful write's refresh.
    context.invalidateRefresh();
    // The page owns its failure/draft scope; a late rejection must not put an
    // old record's error into a different editor. Legacy handlers report below.
    const result = await work();
    if (!context.isDisposed()) await context.refresh();
    return result;
  };
  const deleteRecord = (kind: AutomationKind, id: string, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    const options = checkedOptions(revision);
    return mutateVersioned(async () => {
      if (persistent.value !== false) {
        if (revision === null) throw new ControlCallError('capability_unavailable', 'Read the current definition revision before writing.');
        await definitions.submit({ operation: 'delete', kind, id, expectedRevision: revision }, checkedWriteHost(host));
        return;
      }
      const response = await callWithMeta<{ deleted: boolean }>(control, 'automation.delete', { id, kind }, options);
      const after = recordRevision(response.meta, kind, id, revision === null);
      if (revision !== null && after !== (response.result.deleted ? String(BigInt(revision) + 1n) : revision)) {
        throw new ControlCallError('protocol', 'Unexpected automation delete revision.');
      }
    });
  };
  const setRecordEnabled = (kind: AutomationKind, id: string, enabled: boolean, revision: AutomationRevision, host?: AutomationHost | null): Promise<void> => {
    const options = checkedOptions(revision);
    return mutateVersioned(async () => {
      if (persistent.value !== false) {
        if (revision === null) throw new ControlCallError('capability_unavailable', 'Read the current definition revision before writing.');
        await definitions.submit({ operation: enabled ? 'enable' : 'disable', kind, id, expectedRevision: revision }, checkedWriteHost(host));
        return;
      }
      const response = await callWithMeta<AutomationRecord>(control, enabled ? 'automation.enable' : 'automation.disable', { id, kind }, options);
      assertRecord(response.result, kind, id);
      const after = recordRevision(response.meta, kind, id, revision === null);
      if (response.result.enabled !== enabled || (revision !== null && after !== String(BigInt(revision) + 1n))) {
        throw new ControlCallError('protocol', 'Unexpected automation enablement acknowledgement.');
      }
    });
  };

  const saveRecordAndRefresh = async (
    kind: AutomationKind,
    record: AutomationRecord,
    base: AutomationWriteBase,
  ): Promise<AutomationRecordVersion> => {
    const options = checkedOptions(base.revision);
    return mutateVersioned(async () => {
      if (persistent.value !== false) {
        if (base.revision === null) throw new ControlCallError('capability_unavailable', 'Read the current definition revision before writing.');
        const host = checkedWriteHost(base.host);
        const response = await definitions.submit({ operation: base.create ? 'create' : 'update', kind, id: record.id, record, expectedRevision: base.revision }, host);
        assertRecord(response.value as AutomationRecord, kind, record.id);
        return { record: response.value as AutomationRecord, revision: response.resourceVersions[0]!.revision, host };
      }
      const method = base.create ? 'automation.create' : 'automation.update';
      const response = await callWithMeta<AutomationRecord>(control, method,
        base.create ? { kind, record } : { id: record.id, kind, record }, options);
      assertRecord(response.result, kind, record.id);
      const revision = recordRevision(response.meta, kind, record.id, base.revision === null);
      if (base.revision !== null && revision !== String(BigInt(base.revision) + 1n)) {
        throw new ControlCallError('protocol', 'Unexpected automation save revision.');
      }
      return { record: response.result, revision };
    });
  };

  return { checkedOptions, readRecord, deleteRecord, setRecordEnabled, saveRecordAndRefresh };
}
