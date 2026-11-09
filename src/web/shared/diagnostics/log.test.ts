import { describe, expect, test } from 'vitest';
import { createDiagnosticLog, formatDiagnostics, redactDiagnostic } from './log.ts';
const failure = { source: 'plugins', level: 'error', code: 'health.failed', entityId: 'game', message: 'unreachable' } as const;
describe('diagnostic retention and redaction', () => {
  test('redacts before retaining, searching, deduplicating and copying', () => {
    const log = createDiagnosticLog();
    log.record({ ...failure, message: 'token=abc123 password="with spaces" known-private', details: 'https://user:pass@example.test/api?key=private#secret\nAuthorization: Bearer abc\nCookie: session=value' }, ['known-private']);
    const exported = formatDiagnostics(log.entries.value);
    for (const secret of ['abc123', 'with spaces', 'known-private', 'user:pass', 'key=private', '#secret', 'Bearer abc', 'session=value']) expect(exported).not.toContain(secret);
    expect(exported).toContain('example.test/api');
    expect(exported).toContain('[redacted');
  });
  test('groups bursts by source and entity, preserves first/last times, separates recovery', () => {
    let now = 100;
    const log = createDiagnosticLog({ now: () => now });
    const first = log.record(failure); now += 1000;
    expect(log.record(failure).id).toBe(first.id);
    expect(log.entries.value[0]).toMatchObject({ firstSeenAt: 100, lastSeenAt: 1100, repeatCount: 2 });
    log.record({ ...failure, entityId: 'another' });
    log.record({ ...failure, message: 'recovered', level: 'info' });
    log.record(failure);
    expect(log.entries.value).toHaveLength(4);
    expect(log.latest('plugins', 'game')?.message).toBe('unreachable');
  });
  test('bounds count and UTF-8 bytes and clears counters', () => {
    const log = createDiagnosticLog({ capacity: 2 });
    for (let i = 0; i < 4; i++) log.record({ ...failure, message: String(i) });
    expect(log.entries.value.map(e => e.message)).toEqual(['2', '3']);
    expect(log.dropped.value).toBe(2);
    log.clear(); expect(log.unread.value).toBe(0); expect(log.dropped.value).toBe(0);
    const tiny = createDiagnosticLog({ maxBytes: 400 });
    tiny.record({ ...failure, message: '😀'.repeat(200) });
    expect(tiny.entries.value).toHaveLength(0);
    expect(tiny.dropped.value).toBe(1);
  });
  test('redacts before truncating so long secret fields cannot leak', () => {
    const secret = 'x'.repeat(5000);
    expect(redactDiagnostic(`password="${secret}"`)).toBe('password=[redacted]');
  });
});
