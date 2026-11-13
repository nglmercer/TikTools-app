import ts from 'typescript';
import { parse } from 'vue/compiler-sfc';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { componentFamilies } from './component-families.ts';
const root = resolve(import.meta.dirname, '../..');
const check = process.argv.includes('--check');
async function output(path: string, value: string) {
  const target = resolve(root, path);
  if (check) { if (await readFile(target, 'utf8').catch(() => '') !== value) throw new Error(`Stale generated reference: ${path}`); }
  else { await mkdir(resolve(target, '..'), { recursive: true }); await writeFile(target, value); }
}
const cell = (value: string) => '`' + value.replaceAll('|', '&#124;').replaceAll('\n', ' ').replaceAll('`', '') + '`';
for (const [family, definition] of Object.entries(componentFamilies)) {
  const apis: Array<{ path: string; props: string[][]; events: string[][]; models: string[][]; slots: string[] }> = [];
  for (const name of definition.components) {
    const path = `src/web/shared/ui/${name}.vue`;
    const source = await readFile(resolve(root, path), 'utf8');
    const { descriptor } = parse(source);
    const script = ts.createSourceFile(path + '.ts', descriptor.scriptSetup?.content ?? descriptor.script?.content ?? '', ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const props: string[][] = [], events: string[][] = [], models: string[][] = [];
    const defaults = new Map<string, string>();
    function visit(node: ts.Node): void {
      if (ts.isCallExpression(node)) {
        const macro = node.expression.getText(script);
        if (macro === 'withDefaults' && node.arguments[1] && ts.isObjectLiteralExpression(node.arguments[1])) for (const item of node.arguments[1].properties) if (ts.isPropertyAssignment(item)) defaults.set(item.name.getText(script), item.initializer.getText(script));
        if (macro === 'defineProps' || macro === 'defineEmits') {
          const type = node.typeArguments?.[0];
          if (type && ts.isTypeLiteralNode(type)) for (const member of type.members) if (ts.isPropertySignature(member) && member.name) (macro === 'defineProps' ? props : events).push([member.name.getText(script), member.type?.getText(script) ?? 'unknown', member.questionToken ? 'optional' : 'required']);
        }
        if (macro === 'defineModel') models.push([node.arguments[0] && ts.isStringLiteral(node.arguments[0]) ? node.arguments[0].text : 'modelValue', node.typeArguments?.[0]?.getText(script) ?? 'unknown', node.arguments.at(-1)?.getText(script) ?? '']);
      }
      ts.forEachChild(node, visit);
    }
    visit(script);
    for (const prop of props) prop.push(defaults.get(prop[0]!) ?? (prop[2] === 'required' ? '—' : 'undefined / Vue boolean false'));
    const slots = [...new Set([...(descriptor.template?.content ?? '').matchAll(/<slot\b([^>]*)/g)].map((match) => /\bname="([^"]+)"/.exec(match[1]!)?.[1] ?? (/\b:name=/.test(match[1]!) ? 'dynamic' : 'default')))];
    apis.push({ path, props, events, models, slots });
  }
  for (const locale of ['en', 'es'] as const) {
    const es = locale === 'es';
    let fragment = '';
    for (const api of apis) {
      fragment += `\n### ${api.path.split('/').at(-1)}\n\n[${es ? 'Código fuente' : 'Source'}](https://github.com/nglmercer/TikTools-app/blob/remake/${api.path})\n\n`;
      fragment += `| ${es ? 'Propiedad' : 'Property'} | ${es ? 'Tipo' : 'Type'} | ${es ? 'Presencia' : 'Presence'} | Default |\n| --- | --- | --- | --- |\n`;
      fragment += api.props.length ? api.props.map((p) => `| ${cell(p[0]!)} | ${cell(p[1]!)} | ${p[2] === 'required' ? (es ? 'Obligatoria' : 'Required') : (es ? 'Opcional' : 'Optional')} | ${cell(p[3]!)} |`).join('\n') + '\n' : `| — | — | — | — |\n`;
      fragment += `\n${es ? 'Eventos propios (los atributos de control nativo se reenvían según el componente)' : 'Own events (native control attributes are forwarded according to the component)'}:\n\n`;
      fragment += api.events.length ? api.events.map((p) => `- ${cell(p[0]!)}: ${cell(p[1]!)}`).join('\n') + '\n' : (es ? 'Sin eventos propios declarados.\n' : 'No own declared events.\n');
      fragment += `\n${es ? 'Modelos' : 'Models'}:\n\n` + (api.models.length ? api.models.map((p) => `- ${cell(p[0]!)}: ${cell(p[1]!)}, ${cell(p[2]!)}`).join('\n') + '\n' : (es ? 'Sin v-model propio.\n' : 'No own v-model.\n'));
      fragment += `\nSlots: ${api.slots.map(cell).join(', ') || '—'}.\n`;
    }
    await output(`docs/.generated/components/${family}.${locale}.md`, fragment);
  }
}
const schema = JSON.parse(await readFile(resolve(root, 'docs/contracts/control-api.json'), 'utf8')) as { methods: Array<{ name: string; description: string; sideEffect: boolean; destructive: boolean; requiresDesktop: boolean; concurrency?: string }> };
for (const locale of ['en', 'es']) {
  let text = `| ${locale === 'es' ? 'Método' : 'Method'} | ${locale === 'es' ? 'Efectos' : 'Effects'} | ${locale === 'es' ? 'Destructivo' : 'Destructive'} | Desktop | CAS |\n| --- | --- | --- | --- | --- |\n`;
  text += schema.methods.map((m) => `| ${cell(m.name)} | ${m.sideEffect} | ${m.destructive} | ${m.requiresDesktop} | ${m.concurrency ?? '—'} |`).join('\n') + '\n';
  await output(`docs/.generated/control-api.${locale}.md`, text);
}
const eventSchema = JSON.parse(await readFile(resolve(root, 'src/automation/contracts/generated/automation-events.schema.json'), 'utf8'));
await output('docs/.generated/schema-provenance.json', JSON.stringify({ controlMethods: schema.methods.length, controlSha256: createHash('sha256').update(await readFile(resolve(root, 'docs/contracts/control-api.json'))).digest('hex'), eventDefinitions: Object.keys(eventSchema.$defs).sort() }, null, 2) + '\n');
console.log(`Verified/generated ${Object.keys(componentFamilies).length} component families and ${schema.methods.length} RPC methods.`);
