import { describe, expect, it } from 'vitest';
import {
  canCopySourceUrl,
  GATEWAY_DEFAULT_PORT,
  gatewayTone,
  isOverlayKind,
  parseGatewayStatus,
  REDACTED_TOKEN,
  redactedSourceUrl,
} from '../../../../../../src/web/entities/overlay/lib/overlay-host.ts';

describe('overlay host', () => {
  it('never puts credential material in the displayed source url', () => {
    expect(redactedSourceUrl(17452, 'follow')).toBe(`http://127.0.0.1:17452/widgets/follow/#token=${REDACTED_TOKEN}`);
    expect(redactedSourceUrl(19999, 'gift')).toBe('http://127.0.0.1:19999/widgets/gift/#token=••••••••');
    expect(redactedSourceUrl(17452, 'chat')).not.toContain('?token=');
  });

  it('passes valid gateway payloads through', () => {
    expect(parseGatewayStatus({ state: 'ready', port: 19999, error: null })).toEqual({ state: 'ready', port: 19999, error: null });
    expect(parseGatewayStatus({ state: 'starting', port: 17452, error: 'retry' })).toEqual({
      state: 'starting',
      port: 17452,
      error: 'retry',
    });
  });

  it('coerces garbage into an unreachable status with safe defaults', () => {
    const fallback = { state: 'unreachable', port: GATEWAY_DEFAULT_PORT, error: null };
    expect(parseGatewayStatus(null)).toEqual(fallback);
    expect(parseGatewayStatus({ state: 'melted', port: 0, error: '  ' })).toEqual(fallback);
    expect(parseGatewayStatus({ state: 'ready', port: 99999 })).toEqual({ ...fallback, state: 'ready' });
  });

  it('allows copying unless the credential is known to be missing', () => {
    expect(canCopySourceUrl(null)).toBe(false);
    expect(canCopySourceUrl('missing')).toBe(false);
    expect(canCopySourceUrl('credential-unavailable')).toBe(false);
    expect(canCopySourceUrl('stopped')).toBe(true);
    expect(canCopySourceUrl('ready')).toBe(true);
  });

  it('maps gateway states to a display tone', () => {
    expect(gatewayTone('ready')).toBe('ok');
    expect(gatewayTone('starting')).toBe('busy');
    expect(gatewayTone(null)).toBe('busy');
    expect(gatewayTone('disabled')).toBe('off');
    expect(gatewayTone('unreachable')).toBe('error');
  });

  it('recognises overlay kinds', () => {
    expect(isOverlayKind('gift')).toBe(true);
    expect(isOverlayKind('banner')).toBe(false);
  });
});
