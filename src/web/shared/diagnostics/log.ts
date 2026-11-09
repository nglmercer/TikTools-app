import { shallowRef, ref } from 'vue';

export type DiagnosticLevel = 'info' | 'warning' | 'error';
export type DiagnosticSource = 'live' | 'monitor' | 'discovery' | 'plugins' | 'account' | 'sync' | 'updates' | 'transport';
export interface DiagnosticInput {
  source: DiagnosticSource;
  level: DiagnosticLevel;
  code: string;
  message: string;
  entityId?: string;
  details?: string;
}
export interface DiagnosticEntry extends DiagnosticInput {
  id: string;
  entityId: string;
  details: string;
  firstSeenAt: number;
  lastSeenAt: number;
  repeatCount: number;
}

/** Sanitize before retention. Never pass RPC params, settings or response bodies here. */
export function redactDiagnostic(text: string, secrets: readonly string[] = []): string {
  let result = text;
  for (const secret of secrets) if (secret.length >= 4) result = result.split(secret).join('[redacted]');
  result = result
    .replace(/https?:\/\/[^\s<>"']+/gi, (raw) => {
      try {
        const url = new URL(raw);
        const hadPrivateParts = Boolean(url.username || url.password || url.search || url.hash);
        url.username = ''; url.password = ''; url.search = ''; url.hash = '';
        return url.toString() + (hadPrivateParts ? ' [redacted URL data]' : '');
      } catch { return '[redacted URL]'; }
    })
    .replace(/\b(?:authorization|proxy-authorization|cookie|set-cookie)\s*[:=][^\r\n]*/gi, '[redacted header]')
    .replace(/\b(?:bearer|basic)\s+[A-Za-z0-9+/_=.-]+/gi, '[redacted credential]')
    .replace(/(["']?\b(?:api[_-]?key|api[_-]?token|access[_-]?token|refresh[_-]?token|session[_-]?(?:cookie|id)|token|password|secret|client[_-]?secret)["']?\s*[:=]\s*)(?:"[^"\r\n]*"|'[^'\r\n]*'|[^\s,;}]+)/gi, '$1[redacted]')
    .replace(/\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/g, '[redacted JWT]')
    // Strip non-printing controls from untrusted diagnostic text.
    // eslint-disable-next-line no-control-regex
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, '');
  return result.length > 4000 ? result.slice(0, 4000) + '…' : result;
}

export function createDiagnosticLog(options: { capacity?: number; maxBytes?: number; now?: () => number } = {}) {
  const entries = shallowRef<DiagnosticEntry[]>([]);
  const dropped = ref(0);
  const unread = ref(0);
  let sequence = 0;
  const sizes = new Map<string, number>();
  let bytes = 0;
  const capacity = options.capacity ?? 2000;
  const maxBytes = options.maxBytes ?? 2 * 1024 * 1024;
  const now = options.now ?? Date.now;
  const encoder = new TextEncoder();
  function record(input: DiagnosticInput, secrets: readonly string[] = []): DiagnosticEntry {
    const at = now();
    const safe = {
      source: input.source, level: input.level,
      code: redactDiagnostic(input.code, secrets).slice(0, 100),
      entityId: redactDiagnostic(input.entityId ?? '', secrets).slice(0, 160),
      message: redactDiagnostic(input.message, secrets), details: redactDiagnostic(input.details ?? '', secrets),
    };
    const previous = entries.value.findLast((e) => e.source === safe.source && e.entityId === safe.entityId);
    const duplicate = previous && at - previous.lastSeenAt <= 5000 &&
      previous.code === safe.code && previous.level === safe.level && previous.message === safe.message && previous.details === safe.details;
    const entry: DiagnosticEntry = duplicate
      ? { ...previous, lastSeenAt: at, repeatCount: previous.repeatCount + 1 }
      : { ...safe, id: `diagnostic-${++sequence}`, firstSeenAt: at, lastSeenAt: at, repeatCount: 1 };
    if (duplicate) bytes -= sizes.get(entry.id) ?? 0;
    const size = encoder.encode(JSON.stringify(entry)).byteLength;
    sizes.set(entry.id, size); bytes += size;
    const next = duplicate ? entries.value.filter((e) => e.id !== entry.id) : [...entries.value];
    next.push(entry);
    while (next.length > capacity || bytes > maxBytes) {
      const removed = next.shift();
      if (!removed) break;
      bytes -= sizes.get(removed.id) ?? 0; sizes.delete(removed.id); dropped.value++;
    }
    entries.value = next;
    if (input.level !== 'info' && !duplicate) unread.value++;
    return entry;
  }
  function latest(source: DiagnosticSource, entityId = ''): DiagnosticEntry | undefined {
    return entries.value.findLast((e) => e.source === source && e.entityId === entityId);
  }
  function clear(): void { entries.value = []; sizes.clear(); bytes = 0; dropped.value = 0; unread.value = 0; }
  function markRead(): void { unread.value = 0; }
  return { entries, dropped, unread, record, latest, clear, markRead };
}

// One UI session. No localStorage, disk writes, raw console interception or credentials.
export const diagnosticLog = createDiagnosticLog();
export const diagnosticView = {
  open: ref(false), request: ref(0), selectedId: ref(''), source: ref(''), entityId: ref(''),
};
export function formatDiagnostics(entries: readonly DiagnosticEntry[]): string {
  return ['TikTools diagnostics — current UI session — sensitive fields redacted', ...entries.map((e) =>
    `${new Date(e.firstSeenAt).toISOString()} [${e.level}] ${e.source}${e.entityId ? `/${e.entityId}` : ''} ${e.code}\n${e.message}${e.details ? `\n${e.details}` : ''}\nRepeated: ${e.repeatCount}; last: ${new Date(e.lastSeenAt).toISOString()}`,
  )].join('\n\n');
}
