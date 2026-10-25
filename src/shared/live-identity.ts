/** Canonical replay identity. Never infer identity from payload similarity. */
export const LIVE_IDENTITY_TTL_MS = 300_000;
export const LIVE_IDENTITY_CAPACITY = 4096;

export function liveMessageIdentity(type: string, msgId: unknown, room = '', creator = ''): string | null {
  if (typeof msgId !== 'string' || !msgId.trim() || /^0+$/.test(msgId.trim())) return null;
  return JSON.stringify([creator, room, type, msgId.trim()]);
}
