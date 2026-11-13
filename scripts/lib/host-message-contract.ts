import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { GENERATED_EVENT_REGISTRY } from '../../src/automation/contracts/generated/event-registry.generated.ts';

/** Test/tooling only: read display keys from their owners, never duplicate native schemas. */
export function hostMessageContract(): Record<string, string> {
  const source = readFileSync(resolve(import.meta.dirname, '../../crates/tiktools-core/src/services/catalog.rs'), 'utf8');
  const messages: Record<string, string> = {};
  const string = String.raw`("(?:[^"\\]|\\.)*")`;
  const add = (key: string, value: string) => {
    if (messages[key] !== undefined && messages[key] !== value) throw new Error(`Conflicting native label: ${key}`);
    messages[key] = value;
  };
  for (const match of source.matchAll(new RegExp(String.raw`\btext\(\s*${string}\s*,\s*${string}\s*\)`, 'g'))) {
    add(JSON.parse(match[2]!) as string, JSON.parse(match[1]!) as string);
  }
  const field = new RegExp(String.raw`\bfield\(\s*${string}\s*,\s*${string}\s*,\s*${string}\s*,\s*${string}\s*,\s*${string}`, 'g');
  for (const match of source.matchAll(field)) add(JSON.parse(match[5]!) as string, JSON.parse(match[2]!) as string);
  for (const event of Object.values(GENERATED_EVENT_REGISTRY.events)) {
    for (const field of event.fields) {
      const key = 'i18key' in field && typeof field.i18key === 'string' ? field.i18key : `automation.event.field.${field.path}.label`;
      add(key, field.label.en);
    }
  }
  return messages;
}
