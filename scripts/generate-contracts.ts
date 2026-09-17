import { mkdir, readdir, readFile, rm, writeFile } from 'node:fs/promises';
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

type RegistryFieldJson = {
  path: string;
  tsType: string;
  kind: string;
  optional: boolean;
  label: { en: string; es: string };
  hint: { en: string; es: string };
  sample: unknown;
  sourceField?: string;
  sourceMethod?: string;
  sourcePath?: string;
  sourceJsonPath?: string;
  sourceProtoType?: string;
  sourceCardinality?: string;
  sourceTransform?: string;
};

type NativeSource = {
  method: string;
  path: string;
  transform: string;
  jsonPath?: string;
  protoType?: string;
  cardinality?: string;
};

/**
 * Protobuf provenance stamped by `tiktools-core::contracts::tiktok` as
 * `x-native-source`. Absent for TikTools-only fields (`method`, `msgId`,
 * `isHistory`, `streakable`, `giftIconUrl`), which must never claim a source.
 */
function nativeSourceFor(
  contract: string | undefined,
  field: string,
  schema: JsonRecord,
): NativeSource | undefined {
  if (!contract) return undefined;
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const properties = schemaObject(defs[contract]).properties ?? {};
  const property = properties[field];
  if (!isRecord(property)) return undefined;
  const source = property['x-native-source'];
  if (!isRecord(source)) return undefined;
  const { method, path, transform, jsonPath, protobufType, cardinality } = source;
  if (typeof method !== 'string' || typeof path !== 'string' || typeof transform !== 'string') {
    return undefined;
  }
  return {
    method,
    path,
    transform,
    ...(typeof jsonPath === 'string' ? { jsonPath } : {}),
    ...(typeof protobufType === 'string' ? { protoType: protobufType } : {}),
    ...(typeof cardinality === 'string' ? { cardinality } : {}),
  };
}

function unwrapOptionalVariant(schema: Schema, root: JsonRecord): Schema | undefined {
  const resolved = resolveSchema(schema, root);
  if (typeof resolved === 'boolean') return undefined;
  const variants = resolved.anyOf ?? resolved.oneOf;
  if (Array.isArray(variants)) {
    return variants.find((variant) => kindForSchema(variant, root) !== 'null');
  }
  if (Array.isArray(resolved.type)) {
    const nonNull = resolved.type.find((type) => type !== 'null');
    return nonNull === undefined ? undefined : { ...resolved, type: nonNull };
  }
  return undefined;
}

/**
 * Flattens the stable `EventIntel` schema into dotted `event.intel.*`
 * filter/template paths. Object nodes recurse; arrays and scalars become
 * leaves. The free-form provider namespace is skipped: only stable
 * host-defined fields belong in the picker.
 */
function flattenIntelPaths(
  schema: Schema,
  root: JsonRecord,
  prefix: string,
  hintPrefix: string,
  out: RegistryFieldJson[],
  depth = 0,
): void {
  if (depth > 8) return;
  const optional = unwrapOptionalVariant(schema, root);
  const resolved = resolveSchema(optional ?? schema, root);
  if (typeof resolved === 'boolean') return;
  if (resolved.$ref) return;
  if (resolved.type === 'object' && resolved.properties) {
    for (const [key, value] of Object.entries(resolved.properties)) {
      if (prefix === 'event.intel' && key === 'providers') continue;
      flattenIntelPaths(value, root, `${prefix}.${key}`, `${hintPrefix}.${key}`, out, depth + 1);
    }
    return;
  }
  const leaf = prefix.split('.').pop() ?? prefix;
  out.push({
    path: prefix,
    tsType: typeNameFromSchema(schema, root),
    kind: kindForSchema(schema, root),
    optional: true,
    label: { en: humanize(leaf), es: humanize(leaf) },
    hint: { en: hintPrefix, es: hintPrefix },
    sample: sampleForSchema(schema, root),
  });
}

function intelFieldsFor(schema: JsonRecord, scope: 'chat' | 'user' | 'processing'): RegistryFieldJson[] {
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const out: RegistryFieldJson[] = [];
  if (scope === 'chat') {
    flattenIntelPaths(schemaObject(defs.EventIntel), schema, 'event.intel', 'EventIntel', out);
    return out;
  }
  if (scope === 'user') {
    flattenIntelPaths(schemaObject(defs.IntelNickname), schema, 'event.intel.user.nickname', 'EventIntel.user.nickname', out);
  }
  flattenIntelPaths(schemaObject(defs.IntelProcessing), schema, 'event.intel.processing', 'EventIntel.processing', out);
  return out;
}

const INTEL_SAMPLE_EVENT = {
  comment: {
    normalized: 'hello there',
    language: { top: 'en', confidence: 0.9 },
    composition: { emojiOnly: false, allCaps: false, elongated: false },
    spam: { score: 0.04, detected: false },
    tts: { text: 'hello there', language: 'en', confidence: 0.9 },
  },
  user: {
    nickname: {
      normalized: 'Viewer Demo',
      tts: { text: 'Viewer Demo' },
    },
  },
};

function isMergeableRecord(value: unknown): value is JsonRecord {
  return isRecord(value);
}

function deepMergeSample(base: unknown, overlay: unknown): unknown {
  if (Array.isArray(base) || Array.isArray(overlay)) return overlay ?? base;
  if (isMergeableRecord(base) && isMergeableRecord(overlay)) {
    const merged: JsonRecord = { ...base };
    for (const [key, value] of Object.entries(overlay)) {
      merged[key] = key in merged ? deepMergeSample(merged[key], value) : value;
    }
    return merged;
  }
  return overlay ?? base;
}

/**
 * Complete `intel` sample generated from the schema so every registry path
 * resolves against its sample event (drift guard). Chat overlays curated
 * representative values on top.
 */
function intelSampleFor(schema: JsonRecord, curated: boolean): unknown {
  const defs = isRecord(schema.$defs) ? schema.$defs : {};
  const generated = sampleForSchema(schemaObject(defs.EventIntel), schema);
  return curated ? deepMergeSample(generated, INTEL_SAMPLE_EVENT) : generated;
}

const EVENT_REGISTRY_VERSION = 8;

// Sentinels replaced with shared sample references after stringifying, so
// the identical `intel` sample is emitted once instead of per event. The
// runtime value is unchanged; `sampleEventForType` deep-clones on read.
const INTEL_CHAT_SENTINEL = '@@TIKTOOLS_INTEL_CHAT@@';
const INTEL_DEFAULT_SENTINEL = '@@TIKTOOLS_INTEL_DEFAULT@@';

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
    // Protobuf-backed contracts name their vendor message (`x-native-method`);
    // that is the native source, while `dataInterface` keeps the normalized
    // TikTools DTO. App-only contracts keep the DTO as their source.
    const definition = contractName ? defs[contractName] : undefined;
    const rawNativeMethod = isRecord(definition) ? definition['x-native-method'] : undefined;
    const nativeMethod = typeof rawNativeMethod === 'string' ? rawNativeMethod : undefined;
    const required = new Set(contract.required ?? []);
    const fields = Object.entries(contract.properties ?? {}).map(([key, value]) => {
      const native = nativeSourceFor(contractName, key, schema);
      return {
        path: `event.data.${key}`,
        tsType: typeNameFromSchema(value, schema),
        kind: kindForSchema(value, schema),
        optional: !required.has(key),
        label: { en: humanize(key), es: humanize(key) },
        hint: { en: `${contractName ?? 'JsonObject'}.${key}`, es: `${contractName ?? 'JsonObject'}.${key}` },
        sample: sampleForField(key, value, schema),
        sourceField: key,
        ...(native
          ? {
            sourceMethod: native.method,
            sourcePath: native.path,
            sourceTransform: native.transform,
            ...(native.jsonPath !== undefined ? { sourceJsonPath: native.jsonPath } : {}),
            ...(native.protoType !== undefined ? { sourceProtoType: native.protoType } : {}),
            ...(native.cardinality !== undefined ? { sourceCardinality: native.cardinality } : {}),
          }
          : {}),
      };
    });
    const hasUser = eventType.startsWith('tiktok.') && !['tiktok.room_stats', 'tiktok.connected', 'tiktok.disconnected'].includes(eventType);
    const intelScope = eventType === 'tiktok.chat' ? 'chat' : hasUser ? 'user' : 'processing';
    const intelFields = intelFieldsFor(schema, intelScope);
    const sampleData: JsonRecord = {};
    for (const field of fields) sampleData[field.path.slice('event.data.'.length)] = field.sample;
    events[eventType] = {
      dataInterface: contractName ?? 'JsonObject',
      sourceInterface: nativeMethod ?? contractName ?? '-',
      sampleEvent: {
        id: 'sample-event',
        type: eventType,
        timestamp: 0,
        ...(hasUser ? { user: { uniqueId: 'usuario_demo', nickname: 'Viewer Demo', secUid: '', userId: '1' } } : {}),
        data: sampleData,
        intel: eventType === 'tiktok.chat' ? INTEL_CHAT_SENTINEL : INTEL_DEFAULT_SENTINEL,
      },
      fields: [...(hasUser ? envelopePaths : []), ...fields, ...intelFields],
      sourceFields: fields.map((field) => ({ name: field.sourceField, tsType: field.tsType, optional: field.optional })),
      note: `Generated from ${contractName ?? 'the automation envelope'} JSON Schema.`,
    };
  }
  const body = JSON.stringify(
    {
      version: EVENT_REGISTRY_VERSION,
      generatedBy: 'tiktools-core automation contracts',
      generatedFrom: [
        'crates/tiktools-core/src/contracts',
        'src/automation/contracts/generated/automation-events.schema.json',
      ],
      events,
    },
    null,
    2,
  )
    .split(`"${INTEL_CHAT_SENTINEL}"`)
    .join('INTEL_SAMPLE_CHAT')
    .split(`"${INTEL_DEFAULT_SENTINEL}"`)
    .join('INTEL_SAMPLE_DEFAULT');
  const chatSample = JSON.stringify(intelSampleFor(schema, true), null, 2);
  const defaultSample = JSON.stringify(intelSampleFor(schema, false), null, 2);
  return `// THIS FILE IS GENERATED. Run bun run contracts:generate.\n\nexport const EVENT_REGISTRY_VERSION = ${EVENT_REGISTRY_VERSION} as const;\n\nconst INTEL_SAMPLE_CHAT = ${chatSample} as const;\n\nconst INTEL_SAMPLE_DEFAULT = ${defaultSample} as const;\n\nexport const GENERATED_EVENT_REGISTRY = ${body} as const satisfies Record<string, unknown>;\n`;
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
  // Drop legacy outputs (per-definition re-export files) so stale checkouts
  // converge on the five files above.
  if (!checkMode) {
    for (const entry of await readdir(generatedDirectory)) {
      if (!outputs.has(entry)) await rm(join(generatedDirectory, entry), { force: true });
    }
  } else {
    for (const entry of await readdir(generatedDirectory)) {
      if (!outputs.has(entry)) mismatches.push(entry);
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
