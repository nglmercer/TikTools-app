import { describe, expect, it } from 'vitest';
import { ControlCallError } from './control-client.ts';
import { lifecycleFailure, lifecycleMessageKey } from './cloud-lifecycle.ts';

const fail = (code: string, reason?: string) => new ControlCallError(code, 'host text', reason ? { reason } : undefined);

describe('lifecycle failure mapping', () => {
  it('maps the conflicts the host explains by reason', () => {
    expect(lifecycleFailure(fail('conflict', 'assetReferenced'))).toBe('assetReferenced');
    expect(lifecycleFailure(fail('conflict', 'overlayPinned'))).toBe('overlayPinned');
    expect(lifecycleFailure(fail('conflict', 'alreadyInLibrary'))).toBe('alreadyInLibrary');
    expect(lifecycleFailure(fail('conflict'))).toBe('other');
    expect(lifecycleFailure(fail('conflict', 'somethingNew'))).toBe('other');
  });

  it('separates the account quota from the library quota', () => {
    expect(lifecycleFailure(fail('quota_exceeded'))).toBe('quota');
    expect(lifecycleFailure(fail('quota_exceeded', 'libraryQuota'))).toBe('libraryQuota');
  });

  it('maps a missing Cloud capability for every reason the host reports', () => {
    for (const reason of ['cloud_feature_unsupported', 'capabilities_missing', 'variables_unsupported']) {
      expect(lifecycleFailure(fail('capability_unavailable', reason))).toBe('unsupported');
    }
  });

  it('maps not found, rate limit and sign-in', () => {
    expect(lifecycleFailure(fail('not_found', 'notPublished'))).toBe('notPublished');
    expect(lifecycleFailure(fail('not_found'))).toBe('notFound');
    expect(lifecycleFailure(fail('unavailable', 'rateLimited'))).toBe('rateLimited');
    expect(lifecycleFailure(fail('unavailable'))).toBe('other');
    expect(lifecycleFailure(fail('forbidden'))).toBe('signIn');
  });

  it('never reads a message and treats anything else as other', () => {
    expect(lifecycleFailure(new Error('conflict'))).toBe('other');
    expect(lifecycleFailure('conflict')).toBe('other');
    expect(lifecycleMessageKey('other')).toBeNull();
    expect(lifecycleMessageKey('assetReferenced')).toBe('cloudLifecycle.failure.assetReferenced');
  });
});
