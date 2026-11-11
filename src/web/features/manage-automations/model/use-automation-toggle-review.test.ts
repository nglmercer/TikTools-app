import { describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { AutomationHost } from '@/entities/automation/model/record-versions.ts';
import { useAutomationToggleReview } from './use-automation-toggle-review.ts';

const hostA: AutomationHost = { hostId: 'host-a', hostInstanceId: 'instance-a' };
const hostB: AutomationHost = { hostId: 'host-a', hostInstanceId: 'instance-b' };
const messages: Record<string, string> = {
  'automations.conflict.missingRevision': 'missing revision',
  'automations.conflict.deleted': 'deleted',
  'automations.conflict.alreadyApplied': 'already applied',
};

function fixture(overrides: { revision?: string | undefined; write?: () => Promise<unknown>; read?: () => Promise<unknown> } = {}) {
  const writes: unknown[][] = [];
  const reads: unknown[][] = [];
  const controls = {
    sourceHost: ref<AutomationHost | null>(hostA),
    revisionFor: vi.fn(() => 'revision' in overrides ? overrides.revision : '3'),
    setRecordEnabled: vi.fn(async (...args: unknown[]) => { writes.push(args); return overrides.write?.(); }),
    readRecord: vi.fn(async (...args: unknown[]) => { reads.push(args); return overrides.read?.(); }),
  };
  const review = useAutomationToggleReview({
    controls: controls as never,
    findRecord: (_kind, id) => id === 'present' ? { name: 'Present' } : undefined,
    t: (key) => messages[key] ?? key,
  });
  return { review, controls, writes, reads };
}

describe('automation toggle review', () => {
  it('rejects a record without a revision before opening anything', () => {
    const f = fixture({ revision: undefined });
    f.review.request('event', 'present', true);
    expect(f.review.open.value).toBe(false);
    expect(f.review.error.value).toBe('missing revision');
    expect(f.review.target.value).toBeNull();
  });

  it('writes once against the revision and host captured when it was requested', async () => {
    const f = fixture();
    f.review.request('action', 'present', false);
    f.controls.sourceHost.value = hostB;
    await f.review.confirm();
    expect(f.writes).toEqual([['action', 'present', false, '3', hostA]]);
    expect(f.review.open.value).toBe(false);
  });

  it('does not open a second review while one is open or running', () => {
    const f = fixture();
    f.review.request('event', 'present', true);
    f.review.request('event', 'present', false);
    expect(f.review.target.value?.enabled).toBe(true);
  });

  it('requires a fresh read after a failed write and never writes again without it', async () => {
    const f = fixture({ write: () => Promise.reject(new ControlCallError('revision_conflict', 'Host changed')) });
    f.review.request('event', 'present', true);
    await f.review.confirm();
    expect(f.review.error.value).toBe('Host changed');
    expect(f.review.needsReview.value).toBe(true);
    expect(f.review.open.value).toBe(true);
    await f.review.confirm();
    expect(f.writes).toHaveLength(1);
  });

  it('adopts the current revision and host after reading, and allows one explicit confirmation', async () => {
    let attempts = 0;
    const f = fixture({
      write: () => (attempts++ === 0 ? Promise.reject(new ControlCallError('revision_conflict', 'Host changed')) : Promise.resolve()),
      read: () => Promise.resolve({ record: { name: 'Renamed', enabled: false }, revision: '4', host: hostB }),
    });
    f.review.request('event', 'present', true);
    await f.review.confirm();
    await f.review.review();
    expect(f.review.needsReview.value).toBe(false);
    expect(f.review.satisfied.value).toBe(false);
    expect(f.review.target.value).toMatchObject({ name: 'Renamed', revision: '4', host: hostB });
    await f.review.confirm();
    expect(f.writes.at(-1)).toEqual(['event', 'present', true, '4', hostB]);
  });

  it('reports an operation that was already applied and blocks the write', async () => {
    const f = fixture({
      write: () => Promise.reject(new ControlCallError('revision_conflict', 'Host changed')),
      read: () => Promise.resolve({ record: { name: 'Present', enabled: true }, revision: '4', host: hostA }),
    });
    f.review.request('event', 'present', true);
    await f.review.confirm();
    await f.review.review();
    expect(f.review.satisfied.value).toBe(true);
    expect(f.review.error.value).toBe('already applied');
    expect(f.review.confirmBlocked.value).toBe(true);
    await f.review.confirm();
    expect(f.writes).toHaveLength(1);
  });

  it('keeps the review on a deleted record and on a failed read', async () => {
    const gone = fixture({ read: () => Promise.resolve({ record: null, revision: '4', host: null }) });
    gone.review.request('event', 'present', true);
    await gone.review.review();
    expect(gone.review.error.value).toBe('deleted');
    const broken = fixture({ read: () => Promise.reject(new Error('read failed')) });
    broken.review.request('event', 'present', true);
    await broken.review.review();
    expect(broken.review.error.value).toBe('read failed');
    expect(broken.review.busy.value).toBe(false);
  });

  it('requires a fresh read when a recovery or a library change happens behind an open review', () => {
    const f = fixture();
    f.review.recovered('compare first');
    expect(f.review.needsReview.value).toBe(false);
    f.review.request('event', 'present', true);
    f.review.recovered('compare first');
    expect(f.review.needsReview.value).toBe(true);
    expect(f.review.error.value).toBe('compare first');
    const other = fixture();
    other.review.markNeedsReview();
    other.review.request('event', 'present', true);
    expect(other.review.needsReview.value).toBe(false);
  });
});
