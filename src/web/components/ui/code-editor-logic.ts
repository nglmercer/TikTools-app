export type TemplateSpanMask = { masked: string; spans: { text: string; bare: boolean }[]; sentinel: string };

/**
 * Mask `{{ ... }}` spans so template-aware JSON stays parseable. A span
 * outside strings becomes a quoted placeholder (valid bare value or key); a
 * span inside a string becomes placeholder text that cannot break the string.
 * Unclosed `{{` (mid-typing) is left literal so the doc stays invalid until
 * the span is complete. Matches render-then-parse runtime semantics.
 */
export function maskTemplateSpans(value: string): TemplateSpanMask {
  let sentinel = '__TTPL_';
  while (value.includes(sentinel)) sentinel = `_${sentinel}`;
  const spans: TemplateSpanMask['spans'] = [];
  let masked = '';
  let index = 0;
  let inString = false;
  let escaped = false;
  while (index < value.length) {
    const char = value[index] as string;
    if (inString) {
      if (escaped) {
        masked += char;
        escaped = false;
        index += 1;
        continue;
      }
      if (char === '\\') {
        masked += char;
        escaped = true;
        index += 1;
        continue;
      }
      if (char === '"') {
        masked += char;
        inString = false;
        index += 1;
        continue;
      }
      const end = char === '{' && value[index + 1] === '{' ? value.indexOf('}}', index + 2) : -1;
      if (end !== -1) {
        masked += `${sentinel}${spans.length}__`;
        spans.push({ text: value.slice(index, end + 2), bare: false });
        index = end + 2;
        continue;
      }
      masked += char;
      index += 1;
      continue;
    }
    if (char === '"') {
      masked += char;
      inString = true;
      index += 1;
      continue;
    }
    const end = char === '{' && value[index + 1] === '{' ? value.indexOf('}}', index + 2) : -1;
    if (end !== -1) {
      masked += `"${sentinel}${spans.length}__"`;
      spans.push({ text: value.slice(index, end + 2), bare: true });
      index = end + 2;
      continue;
    }
    masked += char;
    index += 1;
  }
  return { masked, spans, sentinel };
}

/**
 * Restore spans masked by {@link maskTemplateSpans} after pretty-printing.
 * Bare spans restore with their placeholder quotes (back to unquoted template
 * syntax); in-string spans restore the placeholder text only, so a span that
 * fills a whole string keeps its quotes. The sentinel is collision-free for
 * the document, so untouched text never matches.
 */
export function unmaskTemplateSpans(formatted: string, mask: TemplateSpanMask): string {
  let out = formatted;
  mask.spans.forEach((span, spanIndex) => {
    const placeholder = `${mask.sentinel}${spanIndex}__`;
    out = span.bare
      ? out.split(`"${placeholder}"`).join(span.text)
      : out.split(placeholder).join(span.text);
  });
  return out;
}

/**
 * Pretty-print JSON; `null` when the text is not valid JSON. Template spans
 * are masked before parsing and restored verbatim, so `Formatear` never mangles
 * `{{ }}` placeholders.
 */
export function formatJsonText(value: string): string | null {
  const mask = maskTemplateSpans(value);
  let parsed: unknown;
  try {
    parsed = JSON.parse(mask.masked) as unknown;
  } catch {
    return null;
  }
  const formatted = JSON.stringify(parsed, null, 2);
  return mask.spans.length > 0 ? unmaskTemplateSpans(formatted, mask) : formatted;
}

export type JsonValidation =
  | { state: 'empty' }
  | { state: 'valid' }
  | { state: 'invalid'; message: string };

/**
 * Lightweight continuous JSON validation for editors. Runs on every keystroke
 * via native `JSON.parse` over template-masked text, so inline `{{ }}`
 * placeholders validate while true syntax errors still report; callers show
 * the status without blocking input.
 */
export function validateJsonText(value: string): JsonValidation {
  const trimmed = value.trim();
  if (!trimmed) return { state: 'empty' };
  const { masked } = maskTemplateSpans(trimmed);
  try {
    JSON.parse(masked);
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
