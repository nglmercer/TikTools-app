import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { BUILTIN_EVENT_CONTRACTS } from '../src/automation/contracts/events.ts';

type Schema = boolean | SchemaObject;
type SchemaObject = {
  $ref?: string;
  type?: string | string[];
  properties?: Record<string, Schema>;
  required?: string[];
  items?: Schema;
  anyOf?: Schema[];
  oneOf?: Schema[];
  allOf?: Schema[];
  additionalProperties?: boolean | Schema;
  enum?: unknown[];
};

type JsonRecord = Record<string, unknown>;

const repositoryRoot = resolve(import.meta.dir, '..');
const generatedDirectory = resolve(repositoryRoot, 'src/automation/contracts/generated');
const schemaPath = join(generatedDirectory, 'automation-events.schema.json');
const checkMode = process.argv.includes('--check');

function isRecord(value: unknown): value is JsonRecord {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function schemaObject(value: unknown): SchemaObject {
  return isRecord(value) ? value as SchemaObject : {};
}

function refName(ref: string): string {
  return ref.split('/').pop() ?? ref;
}

function resolveSchema(schema: Schema, root: JsonRecord): Schema {
  if (typeof schema !== 'object' || !schema.$ref) return schema;
  const defs = root.$defs;
  if (!isRecord(defs)) return schema;
  return schemaObject(defs[refName(schema.$ref)]);
}

function typeNameFromSchema(schema: Schema, root: JsonRecord): string {
  const resolved = resolveSchema(schema, root);
  if (typeof resolved === 'boolean') return resolved ? 'JsonValue' : 'never';
  if (resolved.$ref) return refName(resolved.$ref);
  if (resolved.enum?.length) {
    return resolved.enum.map((value) => JSON.stringify(value)).join(' | ');
  }
  if (resolved.anyOf || resolved.oneOf) {
    const variants = resolved.anyOf ?? resolved.oneOf ?? [];
    return variants.map((variant) => typeNameFromSchema(variant, root)).join(' | ') || 'JsonValue';
  }
  if (resolved.allOf?.length) {
    return resolved.allOf.map((variant) => typeNameFromSchema(variant, root)).join(' & ');
  }
  if (Array.isArray(resolved.type)) {
    return resolved.type.map((type) => typeNameFromSchema({ type }, root)).join(' | ');
  }
  switch (resolved.type) {
    case 'string': return 'string';
    case 'number':
    case 'integer': return 'number';
    case 'boolean': return 'boolean';
    case 'null': return 'null';
    case 'array': return `${resolved.items ? typeNameFromSchema(resolved.items, root) : 'JsonValue'}[]`;
    case 'object': {
      if (!resolved.properties) return 'JsonObject';
      const fields = Object.entries(resolved.properties).map(([key, value]) => `${JSON.stringify(key)}: ${typeNameFromSchema(value, root)}`);
      return `{ ${fields.join('; ')} }`;
    }
    default: return 'JsonValue';
  }
}

function interfaceForDefinition(name: string, schema: SchemaObject, root: JsonRecord): string {
  const properties = schema.properties ?? {};
  const required = new Set(schema.required ?? []);
  const lines = [`export interface ${name} {`];
  for (const [key, value] of Object.entries(properties)) {
    const optional = required.has(key) ? '' : '?';
    lines.push(`  ${JSON.stringify(key)}${optional}: ${typeNameFromSchema(value, root)};`);
  }
  lines.push('}', '');
  return lines.join('\n');
}

function generatedTypeSource(schema: JsonRecord): string {
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const definitions = Object.entries(defs)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([name, value]) => interfaceForDefinition(name, schemaObject(value), schema))
    .join('\n');
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nimport type { JsonValue } from './json-value.ts';\n\n${definitions}`;
}

function generatedJsonValueSource(): string {
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nexport type JsonPrimitive = null | boolean | number | string;\nexport type JsonValue = JsonPrimitive | JsonObject | JsonArray;\nexport type JsonObject = { [key: string]: JsonValue | undefined };\nexport type JsonArray = JsonValue[];\n`;
}

function fileStem(name: string): string {
  return name
    .replace(/AutomationData$/, '')
    .replace(/Automation$/, '')
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .toLowerCase()
    .replace(/(^|[-_])(.)/g, (_, prefix: string, character: string) => `${prefix}${character.toLowerCase()}`)
    .replace(/^connection$/, 'connection-event')
    .replace(/^points-awarded$/, 'points-awarded-event')
    .replace(/^plugin-emit$/, 'plugin-emit-event')
    .replace(/^room-stats$/, 'room-stats-event')
    .replace(/^chat$/, 'chat-event')
    .replace(/^gift$/, 'gift-event')
    .replace(/^like$/, 'like-event')
    .replace(/^social$/, 'social-event')
    .replace(/^member$/, 'member-event');
}

function generatedReexportSource(name: string): string {
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nexport type { ${name} } from './automation-events.ts';\n`;
}

function generatedIndexSource(names: string[]): string {
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nexport type { JsonArray, JsonObject, JsonPrimitive, JsonValue } from './json-value.ts';\nexport type { ${names.join(', ')} } from './automation-events.ts';\n`;
}

function humanize(value: string): string {
  return value
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/^./, (character) => character.toUpperCase());
}

function kindForSchema(schema: Schema, root: JsonRecord): string {
  const resolved = resolveSchema(schema, root);
  if (typeof resolved === 'boolean') return resolved ? 'unknown' : 'unknown';
  if (resolved.anyOf || resolved.oneOf) {
    const variants = resolved.anyOf ?? resolved.oneOf ?? [];
    const nonNull = variants.find((variant) => typeNameFromSchema(variant, root) !== 'null');
    return nonNull ? kindForSchema(nonNull, root) : 'null';
  }
  if (Array.isArray(resolved.type)) {
    const nonNull = resolved.type.find((type) => type !== 'null');
    return nonNull ? kindForSchema({ type: nonNull }, root) : 'null';
  }
  switch (resolved.type) {
    case 'integer':
    case 'number': return 'number';
    case 'boolean': return 'boolean';
    case 'array': return 'array';
    case 'object': return 'object';
    case 'null': return 'null';
    case 'string': return 'string';
    default: return 'unknown';
  }
}

function sampleForSchema(schema: Schema, root: JsonRecord): unknown {
  const resolved = resolveSchema(schema, root);
  if (typeof resolved === 'boolean') return resolved ? {} : null;
  if (resolved.enum?.length) return resolved.enum[0];
  if (resolved.anyOf || resolved.oneOf) {
    const variants = resolved.anyOf ?? resolved.oneOf ?? [];
    const nonNull = variants.find((variant) => kindForSchema(variant, root) !== 'null');
    return sampleForSchema(nonNull ?? variants[0] ?? true, root);
  }
  switch (Array.isArray(resolved.type) ? resolved.type.find((type) => type !== 'null') : resolved.type) {
    case 'string': return 'sample';
    case 'number':
    case 'integer': return 1;
    case 'boolean': return false;
    case 'array': return [];
    case 'object': {
      const result: JsonRecord = {};
      for (const [key, value] of Object.entries(resolved.properties ?? {})) {
        result[key] = sampleForSchema(value, root);
      }
      return result;
    }
    default: return {};
  }
}

function sampleForField(name: string, schema: Schema, root: JsonRecord): unknown {
  switch (name) {
    case 'giftName': return 'Rosa';
    case 'giftId': return '5655';
    case 'comment': return 'Hello there';
    case 'msgId': return '1';
    case 'method': return 'WebcastSampleMessage';
    case 'emitType': return 'plugin.sample';
    default: return sampleForSchema(schema, root);
  }
}

function registrySource(schema: JsonRecord): string {
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const user = schemaObject(defs.AutomationUser);
  const envelopePaths = Object.entries(user.properties ?? {}).map(([key, value]) => ({
    path: `event.user.${key}`,
    tsType: typeNameFromSchema(value, schema),
    kind: kindForSchema(value, schema),
    optional: !(user.required ?? []).includes(key),
    label: { en: humanize(key), es: humanize(key) },
    hint: { en: `AutomationUser.${key}`, es: `AutomationUser.${key}` },
    sample: sampleForField(key, value, schema),
  }));
  const events: JsonRecord = {};
  for (const [eventType, contractName] of Object.entries(BUILTIN_EVENT_CONTRACTS)) {
    const contract = contractName ? schemaObject(defs[contractName]) : {};
    const required = new Set(contract.required ?? []);
    const fields = Object.entries(contract.properties ?? {}).map(([key, value]) => ({
      path: `event.data.${key}`,
      tsType: typeNameFromSchema(value, schema),
      kind: kindForSchema(value, schema),
      optional: !required.has(key),
      label: { en: humanize(key), es: humanize(key) },
      hint: { en: `${contractName ?? 'JsonObject'}.${key}`, es: `${contractName ?? 'JsonObject'}.${key}` },
      sample: sampleForField(key, value, schema),
      sourceField: key,
    }));
    const hasUser = eventType.startsWith('tiktok.') && !['tiktok.room_stats', 'tiktok.connected', 'tiktok.disconnected'].includes(eventType);
    const sampleData: JsonRecord = {};
    for (const field of fields) sampleData[field.path.slice('event.data.'.length)] = field.sample;
    events[eventType] = {
      dataInterface: contractName ?? 'JsonObject',
      sourceInterface: contractName ?? '-',
      sampleEvent: {
        id: 'sample-event',
        type: eventType,
        timestamp: 0,
        ...(hasUser ? { user: { uniqueId: 'usuario_demo', nickname: 'Viewer Demo', secUid: '', userId: '1' } } : {}),
        data: sampleData,
      },
      fields: [...(hasUser ? envelopePaths : []), ...fields],
      sourceFields: fields.map((field) => ({ name: field.sourceField, tsType: field.tsType, optional: field.optional })),
      note: `Generated from ${contractName ?? 'the automation envelope'} JSON Schema.`,
    };
  }
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nexport const EVENT_REGISTRY_VERSION = 3 as const;\nexport const GENERATED_EVENT_REGISTRY = ${JSON.stringify({ version: 3, generatedBy: 'tiktools-core automation contracts', generatedFrom: ['crates/tiktools-core/src/contracts', 'src/automation/contracts/generated/automation-events.schema.json'], events }, null, 2)} as const satisfies Record<string, unknown>;\n`;
}

async function runSchemaGenerator(output: string): Promise<void> {
  const process = Bun.spawn([
    'cargo', 'run', '-q', '-p', 'tiktools-core', '--example', 'generate-contracts', '--locked', '--', output,
  ], { cwd: repositoryRoot, stdout: 'pipe', stderr: 'pipe' });
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  if (exitCode !== 0) throw new Error(`Rust contract generation failed.\n${stdout}\n${stderr}`);
  if (stderr.trim()) console.error(stderr.trim());
}

async function main(): Promise<void> {
  await mkdir(generatedDirectory, { recursive: true });
  let temporaryDirectory: string | undefined;
  const outputSchemaPath = checkMode
    ? (temporaryDirectory = await mkdtemp(join(tmpdir(), 'tiktools-contracts-')), join(temporaryDirectory, 'automation-events.schema.json'))
    : schemaPath;
  await runSchemaGenerator(outputSchemaPath);
  const schema = JSON.parse(await readFile(outputSchemaPath, 'utf8')) as JsonRecord;
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const names = Object.keys(defs).sort();
  const outputs = new Map<string, string>([
    ['automation-events.schema.json', `${JSON.stringify(schema, null, 2)}\n`],
    ['json-value.ts', generatedJsonValueSource()],
    ['automation-events.ts', generatedTypeSource(schema)],
    ['index.ts', generatedIndexSource(names)],
    ['event-registry.generated.ts', registrySource(schema)],
  ]);
  for (const name of names) outputs.set(`${fileStem(name)}.ts`, generatedReexportSource(name));

  const mismatches: string[] = [];
  for (const [file, contents] of outputs) {
    const target = join(generatedDirectory, file);
    if (checkMode) {
      let current = '';
      try { current = await readFile(target, 'utf8'); } catch { /* missing file is a mismatch */ }
      if (current !== contents) mismatches.push(file);
    } else {
      await mkdir(dirname(target), { recursive: true });
      await writeFile(target, contents, 'utf8');
    }
  }
  if (checkMode) {
    if (temporaryDirectory) await rm(temporaryDirectory, { recursive: true, force: true });
    if (mismatches.length) throw new Error(`Generated contract files are stale: ${mismatches.join(', ')}`);
    console.log('Contract generation check passed.');
  } else {
    console.log(`Generated ${outputs.size} automation contract files.`);
  }
}

await main();
