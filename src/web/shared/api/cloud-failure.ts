import { shallowRef, type ShallowRef } from 'vue';
import { ControlCallError, errorMessage } from './control-client.ts';

/**
 * What the host keeps of a Cloud error envelope: a stable code, the correlation id and
 * whether repeating the request is worth trying. It never carries a message or a stack.
 */
export interface CloudFailure {
  readonly code: string;
  readonly requestId?: string;
  readonly retryable?: boolean;
  readonly retryAfterMs?: number;
}

/** Codes the UI has a short message for. Anything else falls back to the host's message. */
export const CLOUD_FAILURE_CODES = [
  'unsupported', 'offline', 'unauthorized', 'forbidden', 'misconfigured',
  'conflict', 'expired', 'rateLimited', 'temporaryFailure',
] as const;
export type CloudFailureCode = (typeof CLOUD_FAILURE_CODES)[number];

export function isKnownCloudFailureCode(code: string): code is CloudFailureCode {
  return (CLOUD_FAILURE_CODES as readonly string[]).includes(code);
}

/** Reads the envelope from a failed call, or `null` when the failure did not come from the Cloud. */
export function cloudFailure(failure: unknown): CloudFailure | null {
  if (!(failure instanceof ControlCallError)) return null;
  const data = failure.data;
  if (typeof data !== 'object' || data === null) return null;
  const cloud = (data as { cloud?: unknown }).cloud;
  if (typeof cloud !== 'object' || cloud === null) return null;
  const { code, requestId, retryable, retryAfterMs } = cloud as Record<string, unknown>;
  if (typeof code !== 'string' || code.length === 0 || code.length > 32) return null;
  return {
    code,
    ...(typeof requestId === 'string' && requestId.length <= 80 ? { requestId } : {}),
    ...(typeof retryable === 'boolean' ? { retryable } : {}),
    ...(typeof retryAfterMs === 'number' && Number.isFinite(retryAfterMs) && retryAfterMs >= 0 ? { retryAfterMs } : {}),
  };
}

/**
 * Keeps the envelope of the latest failure next to the message a store shows. `capture`
 * returns the message to store, so a store replaces `errorMessage(failure)` with it, and
 * `clear` drops the envelope when the store clears the message, so the two never disagree.
 */
export function useCloudFailure(): {
  detail: ShallowRef<CloudFailure | null>;
  capture: (failure: unknown) => string;
  clear: () => void;
} {
  const detail = shallowRef<CloudFailure | null>(null);
  return {
    detail,
    capture(failure) {
      detail.value = cloudFailure(failure);
      return errorMessage(failure);
    },
    clear() {
      detail.value = null;
    },
  };
}

/**
 * Short text for a panel: the localized message for a known code, otherwise the host's
 * message. The protocol detail stays in Diagnostics.
 */
export function failureText(
  t: (key: string) => string,
  detail: CloudFailure | null,
  fallback: string,
): string {
  return detail && isKnownCloudFailureCode(detail.code) ? t(`cloudFailure.${detail.code}`) : fallback;
}
