import type { DocLocale } from '../locales.ts';

export interface ReferenceMethod {
  name: string;
  description: string;
  sideEffect: boolean;
  destructive: boolean;
  requiresDesktop: boolean;
  concurrency?: string;
  durableCommands: boolean;
  paramsSchema: unknown;
  resultSchema: unknown;
}

const labels = {
  en: ['Method', 'Effects', 'Destructive', 'Concurrency', 'Durable commands', 'Parameters', 'Result', 'Contract description'],
  es: ['Método', 'Efectos', 'Destructivo', 'Concurrencia', 'Comandos durables', 'Parámetros', 'Resultado', 'Descripción del contrato'],
  pt: ['Método', 'Efeitos', 'Destrutivo', 'Concorrência', 'Comandos duráveis', 'Parâmetros', 'Resultado', 'Descrição do contrato'],
  fr: ['Méthode', 'Effets', 'Destructif', 'Concurrence', 'Commandes durables', 'Paramètres', 'Résultat', 'Description du contrat'],
  zh: ['方法', '副作用', '破坏性操作', '并发', '持久命令', '参数', '结果', '契约描述'],
  ja: ['メソッド', '副作用', '破壊的操作', '同時実行', '永続コマンド', 'パラメーター', '結果', '契約の説明'],
} as const;

export function renderControlReference(methods: readonly ReferenceMethod[], locale: DocLocale): string {
  const t = labels[locale];
  const anchor = (name: string) => `rpc-${name.replaceAll('.', '-')}`;
  let text = `| ${t[0]} | ${t[1]} | ${t[2]} | Desktop | ${t[3]} | ${t[4]} |\n| --- | --- | --- | --- | --- | --- |\n`;
  text += methods.map((m) => `| [\`${m.name}\`](#${anchor(m.name)}) | ${m.sideEffect} | ${m.destructive} | ${m.requiresDesktop} | ${m.concurrency ?? '—'} | ${m.durableCommands} |`).join('\n') + '\n';
  for (const m of methods) {
    text += `\n### ${m.name} {#${anchor(m.name)}}\n\n${t[7]}:\n\n\`\`\`text\n${m.description}\n\`\`\`\n\n`;
    text += `\`${JSON.stringify({ sideEffect: m.sideEffect, destructive: m.destructive, requiresDesktop: m.requiresDesktop, concurrency: m.concurrency ?? null, durableCommands: m.durableCommands })}\`\n`;
    for (const [label, schema] of [[t[5], m.paramsSchema], [t[6], m.resultSchema]] as const) {
      text += `\n::: details ${label}\n\n\`\`\`json\n${JSON.stringify(schema, null, 2)}\n\`\`\`\n\n:::\n`;
    }
  }
  return text;
}
