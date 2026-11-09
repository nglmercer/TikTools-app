import { capabilities, resourceKinds, type Grant, type Configuration } from '../api/types.ts';
import { validEffectGrants } from './effects.ts';
const bytes = new TextEncoder();
function record(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function id(value: unknown): value is string { return typeof value === 'string' && value.length > 0 && bytes.encode(value).length <= 256 && !value.includes('*') && !/\p{Cc}/u.test(value); }
function ids(value: unknown): value is string[] { return Array.isArray(value) && value.length <= 256 && value.every(id); }
function automationId(value: string): boolean {
  const separator = value.indexOf(':');
  const family = value.slice(0, separator);
  const raw = value.slice(separator + 1);
  return (family === 'action' || family === 'event') && raw.length > 0 && raw.trim() === raw && bytes.encode(raw).length <= 128;
}
function validDelegation(value: unknown, caps: unknown[]): boolean {
  if (!record(value) || Object.keys(value).some((k) => !['effects', 'allowDynamicTargets'].includes(k)) ||
      !Array.isArray(value['effects']) || value['effects'].length === 0 || value['effects'].length > 7 ||
      new Set(value['effects']).size !== value['effects'].length || typeof value['allowDynamicTargets'] !== 'boolean' ||
      !['automation.read', 'automation.edit', 'automation.toggle'].every((cap) => caps.includes(cap))) return false;
  const needed: Record<string, string | null> = { log: null, delay: null, points: 'points.adjust', http: 'http.request', audio: 'audio.play', emit: 'events.emit', script: 'scripts.execute' };
  return value['effects'].every((effect) => typeof effect === 'string' && Object.hasOwn(needed, effect) && (needed[effect] === null || caps.includes(needed[effect])));
}
/** Validate local edits without silently removing unknown grants or restrictions. */
export function parseGrant(input: unknown): Grant {
  if (!record(input) || Object.keys(input).some((key) => !['capabilities', 'resources', 'settingsFields', 'actions', 'maxPointsDelta', 'effects', 'automationDelegation'].includes(key)) ||
    !Array.isArray(input['capabilities']) || input['capabilities'].some((v) => !capabilities.includes(v)) ||
    !record(input['resources']) || Object.keys(input['resources']).some((k) => !resourceKinds.includes(k as typeof resourceKinds[number])) ||
    Object.values(input['resources']).some((v) => !ids(v)) || !record(input['settingsFields']) || !record(input['actions']) ||
    Object.hasOwn(input, 'effects') && !validEffectGrants(input['effects']) ||
    Object.hasOwn(input, 'automationDelegation') && !validDelegation(input['automationDelegation'], input['capabilities'])) throw new Error('invalid_grant');
  const automation = input['resources']['automation'];
  if (Array.isArray(automation) && !automation.every(automationId)) throw new Error('invalid_grant');
  for (const map of [input['settingsFields'], input['actions']]) {
    if (Object.keys(map).length > 256 || Object.entries(map).some(([key, value]) => !id(key) || !ids(value))) throw new Error('invalid_grant');
  }
  if (input['maxPointsDelta'] !== null && (typeof input['maxPointsDelta'] !== 'number' || !Number.isFinite(input['maxPointsDelta']) || input['maxPointsDelta'] <= 0)) throw new Error('invalid_grant');
  return JSON.parse(JSON.stringify(input)) as Grant;
}
export function splitIds(value: string): string[] { return [...new Set(value.split('\n').map((s) => s.trim()).filter(Boolean))]; }
export function prepareConfiguration(input: Configuration): Configuration {
  const result = JSON.parse(JSON.stringify(input)) as Configuration;
  const origin = (value: string): string => {
    const url = new URL(value.trim());
    if (url.protocol !== 'https:' || url.username || url.password || url.pathname !== '/' || url.search || url.hash) throw new Error('invalid_configuration');
    return url.origin;
  };
  result.advertisedOrigin = result.advertisedOrigin?.trim() ? origin(result.advertisedOrigin) : null;
  result.allowedOrigins = [...new Set(result.allowedOrigins.map(origin))];
  result.bindAddress = result.bindAddress.trim();
  result.tlsCertificate = result.tlsCertificate?.trim() || null;
  result.tlsPrivateKey = result.tlsPrivateKey?.trim() || null;
  if (result.tlsProxy) { result.tlsCertificate = null; result.tlsPrivateKey = null; }
  if (result.enabled && (!result.advertisedOrigin || !result.tlsProxy && (!result.tlsCertificate || !result.tlsPrivateKey))) throw new Error('invalid_configuration');
  if (Object.values(result.limits).some((n) => !Number.isSafeInteger(n) || n < 1)) throw new Error('invalid_configuration');
  return result;
}
