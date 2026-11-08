/** Keep these closed policies aligned with tiktools-collaboration/src/effects.rs. */
export const publicEffectRenderFields = [
  'event.id', 'event.type', 'event.timestamp', 'event.user.id', 'event.user.uniqueId', 'event.user.nickname',
  'event.creator.uniqueId', 'event.creator.roomId', 'event.data.comment', 'event.data.count', 'event.data.likeCount',
  'event.data.totalLikeCount', 'event.data.giftId', 'event.data.giftName', 'event.data.diamondCount',
  'event.data.repeatCount', 'event.data.repeatEnd', 'event.data.giftType',
] as const;
const encoder = new TextEncoder();
const size = (s: string) => encoder.encode(s).length;
const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v);
const exact = (v: unknown, keys: string[]): v is Record<string, unknown> => record(v) && Object.keys(v).length === keys.length && keys.every((k) => Object.hasOwn(v, k));
const strings = (v: unknown, max: number, valid: (s: string) => boolean, min = 0): v is string[] => Array.isArray(v) && v.length >= min && v.length <= max && new Set(v).size === v.length && v.every((s) => typeof s === 'string' && valid(s));
const field = (s: string) => s.length > 0 && size(s) <= 256 && s.trim() === s && !s.includes('*') && !/\p{Cc}/u.test(s);
const header = (s: string) => s.length > 0 && size(s) <= 128 && /^[a-z0-9!#$%&'+.^_`|~-]+$/u.test(s) &&
  !['host', 'content-length', 'transfer-encoding', 'connection', 'upgrade', 'proxy-authorization', 'proxy-connection', 'te', 'trailer'].includes(s);
const topic = (s: string) => s.length > 0 && s.length <= 64 && /^[a-z0-9._-]+$/u.test(s);
const file = (s: string) => s.length > 0 && size(s) <= 4096 && s.trim() === s && !/\p{Cc}|[*{}]/u.test(s) &&
  (s.startsWith('/') || s.startsWith('\\\\') || /^[A-Za-z]:[/\\]/u.test(s)) && !s.split(/[/\\]/u).some((p) => p === '.' || p === '..');
function local(host: string): boolean {
  if (host === 'localhost' || host.endsWith('.localhost') || host === '[::1]' || /^\[f[cd][0-9a-f]{2}:/u.test(host) || /^\[fe[89ab][0-9a-f]:/u.test(host)) return true;
  const p = host.split('.').map(Number);
  return p.length === 4 && p.every((n) => Number.isInteger(n) && n >= 0 && n <= 255) &&
    (p[0] === 10 || p[0] === 127 || p[0] === 192 && p[1] === 168 || p[0] === 169 && p[1] === 254 || p[0] === 172 && p[1]! >= 16 && p[1]! <= 31);
}
function origin(s: string, privateNetwork: boolean): boolean {
  try { const u = new URL(s); return u.origin === s && u.pathname === '/' && !u.username && !u.password && !u.search && !u.hash &&
    (u.protocol === 'https:' || u.protocol === 'http:' && privateNetwork && local(u.hostname)); } catch { return false; }
}
function path(s: string): boolean {
  if (!s.startsWith('/') || s.startsWith('//') || size(s) > 2048 || /[^\x21-\x7e]|[%\\?#*{}]/u.test(s)) return false;
  try { const u = new URL(`https://grant.invalid${s}`); return u.hostname === 'grant.invalid' && u.pathname === s && !u.search && !u.hash; } catch { return false; }
}
const budget = (v: unknown, min: number, max: number): boolean => typeof v === 'number' && Number.isSafeInteger(v) && v >= min && v <= max;
/** Deny malformed/unknown restrictions; never discard them into a wider grant. */
export function validEffectGrants(value: unknown): boolean {
  if (!exact(value, ['http', 'audio', 'emitTopics']) || !Array.isArray(value.http) || value.http.length > 32 || !Array.isArray(value.audio) || value.audio.length > 32 || !strings(value.emitTopics, 256, topic)) return false;
  if (value.http.some((r: unknown) => !exact(r, ['origin', 'methods', 'paths', 'queryFields', 'headerNames', 'renderFields', 'bodyFields', 'allowTextBody', 'maxBodyBytes', 'maxResponseBytes', 'allowPrivateNetwork']) ||
    typeof r.allowPrivateNetwork !== 'boolean' || typeof r.origin !== 'string' || !origin(r.origin, r.allowPrivateNetwork) ||
    !strings(r.methods, 7, (m) => ['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'].includes(m), 1) || !strings(r.paths, 64, path, 1) ||
    !strings(r.queryFields, 64, field) || !strings(r.headerNames, 64, header) || !strings(r.renderFields, publicEffectRenderFields.length, (s) => publicEffectRenderFields.includes(s as typeof publicEffectRenderFields[number])) ||
    !strings(r.bodyFields, 256, field) || typeof r.allowTextBody !== 'boolean' || !budget(r.maxBodyBytes, 0, 256 * 1024) || !budget(r.maxResponseBytes, 1, 2 * 1024 * 1024))) return false;
  return value.audio.every((r: unknown) => exact(r, ['fileRefs', 'output', 'maxVolume', 'overlaps']) && strings(r.fileRefs, 256, file, 1) && r.output === 'host-default' &&
    typeof r.maxVolume === 'number' && Number.isFinite(r.maxVolume) && r.maxVolume >= 0 && r.maxVolume <= 1 && strings(r.overlaps, 3, (s) => ['allow', 'restart', 'drop'].includes(s), 1)) && size(JSON.stringify(value)) <= 256 * 1024;
}
