export const variableKinds = ['text', 'number', 'boolean', 'list', 'secret'] as const;
export type RuntimeVariableKind = typeof variableKinds[number];
export type RuntimeVariable =
  | { type: 'text' | 'secret'; value: string }
  | { type: 'number'; value: number; numberText?: string }
  | { type: 'boolean'; value: boolean }
  | { type: 'list'; value: string[] };

export interface RuntimeVariablesSnapshot {
  revision: string;
  variables: Record<string, RuntimeVariable>;
}

export function isRuntimeVariable(value: unknown): value is RuntimeVariable {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  switch (record.type) {
    case 'text': case 'secret': return typeof record.value === 'string';
    case 'number': {
      if (typeof record.value !== 'number' || !Number.isFinite(record.value)) return false;
      if (record.numberText === undefined) return !Number.isInteger(record.value) || Number.isSafeInteger(record.value);
      if (typeof record.numberText !== 'string' || record.numberText.length > 4096) return false;
      try {
        const exact: unknown = JSON.parse(record.numberText);
        return typeof exact === 'number' && Number.isFinite(exact) && exact === record.value;
      } catch { return false; }
    }
    case 'boolean': return typeof record.value === 'boolean';
    case 'list': return Array.isArray(record.value) && record.value.every((item) => typeof item === 'string');
    default: return false;
  }
}

/** Textual rendering stays compatible with {{ globals.key }}. */
export function variableText(variable: RuntimeVariable): string {
  if (variable.type === 'number' && variable.numberText !== undefined) return variable.numberText;
  return typeof variable.value === 'string' ? variable.value : JSON.stringify(variable.value);
}

export function textVariables(globals: Record<string, string>): Record<string, RuntimeVariable> {
  return Object.fromEntries(Object.entries(globals).map(([name, value]) => [name, { type: 'text', value }]));
}
