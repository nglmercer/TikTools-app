export const variableKinds = ['text', 'number', 'boolean', 'list', 'secret'] as const;
export type RuntimeVariableKind = typeof variableKinds[number];
export type RuntimeVariable =
  | { type: 'text' | 'secret'; value: string }
  | { type: 'number'; value: number }
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
    case 'number': return typeof record.value === 'number' && Number.isFinite(record.value);
    case 'boolean': return typeof record.value === 'boolean';
    case 'list': return Array.isArray(record.value) && record.value.every((item) => typeof item === 'string');
    default: return false;
  }
}

/** Textual rendering stays compatible with {{ globals.key }}. */
export function variableText(variable: RuntimeVariable): string {
  return typeof variable.value === 'string' ? variable.value : JSON.stringify(variable.value);
}

export function textVariables(globals: Record<string, string>): Record<string, RuntimeVariable> {
  return Object.fromEntries(Object.entries(globals).map(([name, value]) => [name, { type: 'text', value }]));
}
