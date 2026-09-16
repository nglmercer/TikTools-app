/** Pretty-print JSON; `null` when the text is not valid JSON. */
export function formatJsonText(value: string): string | null {
  try {
    return JSON.stringify(JSON.parse(value) as unknown, null, 2);
  } catch {
    return null;
  }
}

export type JsonValidation =
  | { state: 'empty' }
  | { state: 'valid' }
  | { state: 'invalid'; message: string };

/**
 * Lightweight continuous JSON validation for editors. Runs on every keystroke
 * via native `JSON.parse`; callers show the status without blocking input.
 */
export function validateJsonText(value: string): JsonValidation {
  const trimmed = value.trim();
  if (!trimmed) return { state: 'empty' };
  try {
    JSON.parse(trimmed);
    return { state: 'valid' };
  } catch (error) {
    return {
      state: 'invalid',
      message: error instanceof Error ? error.message : 'Invalid JSON',
    };
  }
}

export type PasteFormatOptions = {
  language: string;
  validateJson: boolean;
  formatJsonOnPaste: boolean;
  pastedText: string;
  currentValue: string;
  selectionStart: number;
  selectionEnd: number;
};

/**
 * Auto-format a paste only when the pasted text itself is valid JSON and it
 * replaces the whole document (empty editor or full selection). Mid-document
 * pastes keep normal insertion so surrounding content is never rewritten.
 */
export function shouldFormatPastedJson(options: PasteFormatOptions): boolean {
  if (options.language !== 'json') return false;
  if (!options.validateJson || !options.formatJsonOnPaste) return false;
  if (validateJsonText(options.pastedText).state !== 'valid') return false;
  if (options.currentValue.trim().length === 0) return true;
  return options.selectionStart === 0 && options.selectionEnd === options.currentValue.length;
}

export type JsonToken = { text: string; cls: string };

/** Fault-tolerant JSON highlighter: broken input while typing still renders. */
export function tokenizeJson(chunk: string): JsonToken[] {
  const out: JsonToken[] = [];
  const pattern = /(\s+)|("(?:[^"\\]|\\.)*"?)|(-?\d[\d._]*)|\b(true|false|null)\b|([{}[\],:])|(.)/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(chunk)) !== null) {
    const [full, ws, str, num, lit, punct] = match;
    if (ws) {
      out.push({ text: full, cls: 'codeed-ws' });
      continue;
    }
    if (str) {
      const rest = chunk.slice(match.index + full.length);
      out.push({ text: full, cls: /^\s*:/.test(rest) ? 'codeed-key' : 'codeed-str' });
      continue;
    }
    if (num) {
      out.push({ text: num, cls: 'codeed-num' });
      continue;
    }
    if (lit) {
      out.push({ text: lit, cls: 'codeed-lit' });
      continue;
    }
    if (punct) {
      out.push({ text: punct, cls: 'codeed-punct' });
      continue;
    }
    out.push({ text: full, cls: 'codeed-plain' });
  }
  return out;
}
