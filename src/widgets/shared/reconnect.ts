/**
 * Exponential-backoff reconnect delays with a ceiling and small jitter.
 * Pure function so the progression is unit-testable.
 */

import { RECONNECT_BASE_MS, RECONNECT_FACTOR, RECONNECT_JITTER_MS, RECONNECT_MAX_MS } from './config.ts';

export interface ReconnectDelayOptions {
  baseMs?: number;
  maxMs?: number;
  factor?: number;
  jitterMs?: number;
  /** Injectable randomness for deterministic tests. Defaults to Math.random. */
  random?: () => number;
}

export function computeReconnectDelay(attempt: number, options: ReconnectDelayOptions = {}): number {
  const baseMs = options.baseMs ?? RECONNECT_BASE_MS;
  const maxMs = options.maxMs ?? RECONNECT_MAX_MS;
  const factor = options.factor ?? RECONNECT_FACTOR;
  const jitterMs = options.jitterMs ?? RECONNECT_JITTER_MS;
  const random = options.random ?? Math.random;
  const clampedAttempt = Math.max(0, Math.floor(attempt));
  const backoff = Math.min(maxMs, baseMs * Math.pow(factor, clampedAttempt));
  return Math.floor(backoff + random() * jitterMs);
}
