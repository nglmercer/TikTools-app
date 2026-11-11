import { describe, expect, it } from 'vitest';
import { cloudFailure, failureText, useCloudFailure } from './cloud-failure.ts';
import { ControlCallError } from './control-client.ts';

const call = (data: unknown) => new ControlCallError('unavailable', 'the host is offline', data);

describe('cloud failure envelope', () => {
  it('reads the validated parts the host kept', () => {
    expect(cloudFailure(call({ cloud: { code: 'offline', requestId: 'req-1', retryable: true, retryAfterMs: 1500 } })))
      .toEqual({ code: 'offline', requestId: 'req-1', retryable: true, retryAfterMs: 1500 });
  });

  it('ignores anything that is not a well formed envelope', () => {
    expect(cloudFailure(new Error('plain'))).toBeNull();
    expect(cloudFailure(call(undefined))).toBeNull();
    expect(cloudFailure(call({ cloud: 'offline' }))).toBeNull();
    expect(cloudFailure(call({ cloud: { code: '' } }))).toBeNull();
    expect(cloudFailure(call({ cloud: { code: 'x'.repeat(33) } }))).toBeNull();
    expect(cloudFailure(call({ cloud: { code: 'offline', requestId: 7, retryable: 'yes', retryAfterMs: -1 } })))
      .toEqual({ code: 'offline' });
  });

  it('captures the detail next to the message and replaces it on the next failure', () => {
    const { detail, capture } = useCloudFailure();
    expect(capture(call({ cloud: { code: 'offline' } }))).toBe('the host is offline');
    expect(detail.value?.code).toBe('offline');
    capture(new Error('later'));
    expect(detail.value).toBeNull();
  });

  it('shows a short message for known codes and the host message otherwise', () => {
    const t = (key: string) => `[${key}]`;
    expect(failureText(t, { code: 'offline' }, 'raw')).toBe('[cloudFailure.offline]');
    expect(failureText(t, { code: 'somethingNew' }, 'raw')).toBe('raw');
    expect(failureText(t, null, 'raw')).toBe('raw');
  });
});
