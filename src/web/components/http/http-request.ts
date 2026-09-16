import type { JsonObject, JsonValue } from '../../../automation/types.ts';

/**
 * Shared HTTP request configuration shape for Behavior `core.fetch` and
 * workflow `action.http`. Consumers support a subset of these fields; the
 * editor enables each field through capability props instead of branching on
 * the consumer.
 */
export type HttpRequestConfig = {
  method?: string;
  url?: string;
  headers?: JsonObject;
  body?: string;
  timeoutMs?: number;
  responseType?: string;
  redirect?: string;
  allowPrivateNetwork?: boolean;
  emitResponseAs?: string;
  bodyMode?: HttpBodyMode;
};

export type HttpBodyMode = 'text' | 'json';

export const HTTP_BODY_MODES: HttpBodyMode[] = ['json', 'text'];

export function normalizeHttpMethod(method: JsonValue | undefined, fallback = 'POST'): string {
  if (typeof method === 'string' && method.trim().length > 0) return method.trim().toUpperCase();
  return fallback;
}

/** Trim a base URL and drop trailing slashes without breaking `https://`. */
export function normalizeBaseUrl(raw: string): string {
  const trimmed = raw.trim();
  const match = /^(https?:\/\/[^/]+)\/*(.*)$/i.exec(trimmed);
  if (!match) return trimmed;
  const origin = match[1] ?? trimmed;
  const rest = match[2] ?? '';
  if (!rest) return origin;
  return `${origin}/${rest}`.replace(/\/+$/, '');
}

/** Case-insensitive header lookup over a config header map. */
export function getHeader(headers: JsonObject | undefined, name: string): string | undefined {
  if (!headers) return undefined;
  const wanted = name.toLowerCase();
  for (const [key, value] of Object.entries(headers)) {
    if (key.toLowerCase() === wanted) return typeof value === 'string' ? value : String(value ?? '');
  }
  return undefined;
}

/** Merge header overrides, replacing existing keys case-insensitively. */
export function buildHttpHeaders(
  base: JsonObject | undefined,
  extra: Record<string, string | undefined>,
): JsonObject {
  const headers: JsonObject = { ...(base ?? {}) };
  for (const [key, value] of Object.entries(extra)) {
    if (value === undefined) continue;
    for (const existing of Object.keys(headers)) {
      if (existing !== key && existing.toLowerCase() === key.toLowerCase()) delete headers[existing];
    }
    headers[key] = value;
  }
  return headers;
}

/** Canonical header-map rendering shared by every HTTP editor. */
export function headerMapToText(value: JsonValue | undefined): string {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
  return Object.entries(value)
    .map(([key, raw]) => `${key}: ${typeof raw === 'string' ? raw : String(raw ?? '')}`)
    .join('\n');
}

/** Canonical `Name: value` per-line header parsing shared by every editor. */
export function parseHeadersText(value: string): JsonObject {
  const headers: JsonObject = {};
  for (const line of value.split('\n')) {
    const separator = line.indexOf(':');
    if (separator <= 0) continue;
    const key = line.slice(0, separator).trim();
    if (key) headers[key] = line.slice(separator + 1).trim();
  }
  return headers;
}

export function suggestContentType(mode: HttpBodyMode): string {
  return mode === 'json' ? 'application/json' : 'text/plain';
}

/**
 * Resolve the body editor mode: an explicit `bodyMode` config value wins,
 * then the Content-Type header, then the consumer default.
 */
export function readHttpBodyMode(config: JsonObject, defaultMode: HttpBodyMode): HttpBodyMode {
  const explicit = config.bodyMode;
  if (explicit === 'json' || explicit === 'text') return explicit;
  const contentType = config.headers && typeof config.headers === 'object' && !Array.isArray(config.headers)
    ? getHeader(config.headers as JsonObject, 'content-type')
    : undefined;
  if (contentType) {
    const normalized = contentType.toLowerCase();
    if (normalized.includes('json')) return 'json';
    if (normalized.includes('text/')) return 'text';
  }
  return defaultMode;
}

function readString(value: JsonValue | undefined): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

function readNumber(value: JsonValue | undefined): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

function readHeaders(value: JsonValue | undefined): JsonObject | undefined {
  return value && typeof value === 'object' && !Array.isArray(value) ? { ...(value as JsonObject) } : undefined;
}

/** Typed read of the HTTP fields inside a behavior/workflow config object. */
export function readHttpConfig(config: JsonObject): HttpRequestConfig {
  return {
    method: readString(config.method),
    url: readString(config.url),
    headers: readHeaders(config.headers),
    body: readString(config.body),
    timeoutMs: readNumber(config.timeoutMs),
    responseType: readString(config.responseType),
    redirect: readString(config.redirect),
    allowPrivateNetwork: config.allowPrivateNetwork === true || config.allowPrivateNetwork === 'true'
      ? true
      : config.allowPrivateNetwork === false || config.allowPrivateNetwork === 'false'
        ? false
        : undefined,
    emitResponseAs: readString(config.emitResponseAs),
    bodyMode: config.bodyMode === 'json' || config.bodyMode === 'text' ? config.bodyMode : undefined,
  };
}
