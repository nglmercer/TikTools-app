import type { JsonObject } from '../../../../automation/types.ts';
import { optionFields } from '../../../../automation/plugins/declarative.ts';
import type { ActionOptionItem } from '../../../../shared/messages.ts';
import type { SchemaOption } from '@/shared/ui/schema-form/lib/schema-fields.ts';

export function optionSources(uiHints: JsonObject | undefined): string[] {
  return [...new Set(optionFields(uiHints).map((field) => field.source))];
}

export function fieldOptionsFor(
  uiHints: JsonObject | undefined,
  actionOptions: Readonly<Record<string, readonly ActionOptionItem[]>>,
): Record<string, SchemaOption[]> {
  const merged: Record<string, SchemaOption[]> = {};
  for (const field of optionFields(uiHints)) {
    const options = actionOptions[field.source];
    if (options?.length) merged[field.key] = options.map((option) => ({ ...option }));
  }
  return merged;
}

const MAX_CONTEXT_ENTRIES = 32;
const MAX_CONTEXT_STRING = 512;
const MAX_CONTEXT_KEY = 128;

/** Flattens a form record into the bounded scalar context accepted by
 * `plugins.options` for contextual option sources. Non-scalar values and
 * oversized strings are dropped, mirroring the host's
 * `sanitize_option_context`. */
export function optionContext(values: JsonObject | undefined): JsonObject | undefined {
  if (!values) return undefined;
  const clean: JsonObject = {};
  let kept = 0;
  for (const [key, value] of Object.entries(values)) {
    if (kept >= MAX_CONTEXT_ENTRIES) break;
    if (!key || key.length > MAX_CONTEXT_KEY) continue;
    if (typeof value === 'string') {
      if (value.length > MAX_CONTEXT_STRING) continue;
      clean[key] = value;
    } else if (typeof value === 'number' || typeof value === 'boolean') {
      clean[key] = value;
    } else {
      continue;
    }
    kept += 1;
  }
  return kept ? clean : undefined;
}

/** Stable fingerprint of an option context so watchers can refetch only
 * when the values that feed `optionSources.*.params` templates change. */
export function optionContextSignature(context: JsonObject | undefined): string {
  if (!context) return '';
  return JSON.stringify(
    Object.keys(context)
      .sort()
      .map((key) => [key, context[key]]),
  );
}
