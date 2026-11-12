export const encoder = new TextEncoder();
export const triggers = new Set(['tiktok.chat', 'tiktok.gift', 'tiktok.like', 'tiktok.follow', 'tiktok.share', 'tiktok.join']);
export const validTopic = (value: string) => value.length > 0 && value.length <= 64 && !/[^a-z0-9._-]/.test(value);
export const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
export function boundedEmitData(value: unknown): value is Record<string, unknown> {
  if (!object(value)) return false;
  const pending: { value: unknown; depth: number }[] = [{ value, depth: 0 }];
  let count = 0;
  while (pending.length) {
    const current = pending.pop()!;
    if (++count > 8192 || current.depth > 32 || (typeof current.value === 'number' && !Number.isFinite(current.value))) return false;
    if (current.value && typeof current.value === 'object') {
      for (const child of Object.values(current.value)) pending.push({ value: child, depth: current.depth + 1 });
    }
  }
  try { return encoder.encode(JSON.stringify(value)).length <= 64 * 1024; } catch { return false; }
}
export function httpHeaders(text: string): Record<string, string> | null {
  if (encoder.encode(text).length > 64 * 1024) return null;
  let parsed: unknown; try { parsed = JSON.parse(text); } catch { return null; }
  if (!object(parsed) || Object.keys(parsed).length > 64 || !Object.entries(parsed).every(([key, value]) => !!key && !/[^!#$%&'*+.^_`|~0-9a-z-]/.test(key) && typeof value === 'string' && encoder.encode(value).length <= 8192)) return null;
  return parsed as Record<string, string>;
}
export function validHttpUrl(text: string, allowPrivate: boolean): boolean {
  if (!text || text.trim() !== text || encoder.encode(text).length > 8192) return false;
  try {
    const url = new URL(text);
    return (url.protocol === 'https:' || (url.protocol === 'http:' && allowPrivate)) && !!url.hostname && !url.username && !url.password && !url.hash;
  } catch { return false; }
}
export const integer = (value: string, max: number): number | null => {
  if (!/^(0|[1-9]\d*)$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed <= max ? parsed : null;
};
export const canonicalId = (value: string) => value.length > 0 && value.trim() === value
  && encoder.encode(value).length <= 128 && !/[\p{Cc}*]/u.test(value);
export const validAudioPath = (value: string) => !!value && value.trim() === value && encoder.encode(value).length <= 4096 && !/[\p{Cc}]/u.test(value) && !value.includes('{{') && !value.includes('}}');
export const parseVolume = (value: string): number | null => {
  if (value.trim() !== value || !/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) return null;
  const n = Number(value); return Number.isFinite(n) && n >= 0 && n <= 1 ? n : null;
};
export const literalViewer = (value: string) => canonicalId(value) && !value.startsWith('@') && !value.includes('{{') && !value.includes('}}');
export const pointsChange = (value: string): number | null => {
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed !== 0 && Math.abs(parsed) <= 1_000_000_000 ? parsed : null;
};
