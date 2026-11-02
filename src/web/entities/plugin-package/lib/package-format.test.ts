import { describe, expect, it } from 'vitest';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { JobSnapshot } from '../api/package-types.ts';
import { asInstallError, installErrorOf, installFailure, reviewFromError } from './install-error.ts';
import {
  formatBytes,
  isActiveJob,
  jobPercent,
  originDetail,
  parseRemoteSelection,
  remoteSelection,
  shortDigest,
} from './package-format.ts';

function job(overrides: Partial<JobSnapshot>): JobSnapshot {
  return { id: 'j', kind: 'install', stage: 'download', status: 'running', createdAt: 0, updatedAt: 0, ...overrides };
}

describe('package formatting', () => {
  it('formats sizes and digests', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
    expect(shortDigest(`sha256:${'ab'.repeat(32)}`)).toBe('abababab…abababab');
  });

  it('describes origins and job progress', () => {
    expect(originDetail({ kind: 'registry', registry: 'official', url: 'https://x' })).toBe('official');
    expect(originDetail({ kind: 'file' })).toBe('');
    expect(jobPercent(job({ progress: { done: 25, total: 100 } }))).toBe(25);
    expect(jobPercent(job({ progress: { done: 25 } }))).toBeNull();
    expect(isActiveJob(job({ status: 'queued' }))).toBe(true);
    expect(isActiveJob(job({ status: 'failed' }))).toBe(false);
  });

  it('round-trips remote selections', () => {
    const key = remoteSelection('official', 'acme.tts');
    expect(parseRemoteSelection(key)).toEqual({ registry: 'official', id: 'acme.tts' });
    expect(parseRemoteSelection('acme.tts')).toBeNull();
    expect(parseRemoteSelection('registry:official/')).toBeNull();
    expect(parseRemoteSelection(null)).toBeNull();
  });
});

describe('install errors', () => {
  it('reads typed errors from control failures', () => {
    const failure = new ControlCallError('install_failed', 'busy', { code: 'busy', message: 'busy' });
    expect(installErrorOf(failure)).toEqual({ code: 'busy', message: 'busy', details: undefined });
    expect(installErrorOf(new Error('x'))).toBeNull();
    expect(installFailure(new Error('boom'))).toEqual({ code: 'unsupported', message: 'boom' });
    expect(asInstallError({ code: 'made-up', message: 'x' })).toBeNull();
  });

  it('extracts the fresh review from consent errors', () => {
    const preview = { consentToken: 't', id: 'acme.tts', version: '1.1.0' };
    expect(reviewFromError({ code: 'consent-required', message: 'review', details: preview })).toEqual(preview);
    expect(reviewFromError({ code: 'busy', message: 'busy', details: preview })).toBeNull();
    expect(reviewFromError({ code: 'consent-required', message: 'review' })).toBeNull();
  });
});
