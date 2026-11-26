import type { DocLocale } from '../locales.ts';

export interface CliVerb {
  name: string;
  usage: string;
  summary: string;
  /** Per-locale translations of `summary`; locales missing a translation are absent. */
  summaries?: Partial<Record<DocLocale, string>>;
  destructive: boolean;
}

export interface CliDomain {
  domain: string;
  summary: string;
  summaries?: Partial<Record<DocLocale, string>>;
  verbs: CliVerb[];
}

export interface CliHelp {
  schema: number;
  usage: string;
  commands: CliDomain[];
  conventions: Array<{
    topic: string;
    topics?: Partial<Record<DocLocale, string>>;
    text: string;
    texts?: Partial<Record<DocLocale, string>>;
  }>;
}

const labels = {
  en: { command: 'Command', summary: 'Summary', destructive: 'destructive', conventions: 'Conventions' },
  es: { command: 'Comando', summary: 'Resumen', destructive: 'destructivo', conventions: 'Convenciones' },
  pt: { command: 'Comando', summary: 'Resumo', destructive: 'destrutivo', conventions: 'Convenções' },
  fr: { command: 'Commande', summary: 'Résumé', destructive: 'destructif', conventions: 'Conventions' },
  zh: { command: '命令', summary: '摘要', destructive: '破坏性', conventions: '约定' },
  ja: { command: 'コマンド', summary: '概要', destructive: '破壊的', conventions: '規約' },
} as const;

const cell = (value: string) => '`' + value.replaceAll('|', '&#124;').replaceAll('`', '') + '`';

/**
 * Renders the CLI command registry as one section per domain with a verb
 * table. Usage lines are parser syntax and stay in English in every locale;
 * summaries come from the CLI's own translation catalogs.
 */
export function renderCliReference(help: CliHelp, locale: DocLocale): string {
  const t = labels[locale];
  const local = (entry: { summary?: string; summaries?: Partial<Record<DocLocale, string>> }) =>
    entry.summaries?.[locale] ?? entry.summary ?? '';
  let text = `\`${help.usage}\`\n`;
  for (const domain of help.commands) {
    text += `\n### \`tiktools ${domain.domain}\` — ${local(domain)}\n\n`;
    text += `| ${t.command} | ${t.summary} |\n| --- | --- |\n`;
    text += domain.verbs.map((verb) => {
      const marker = verb.destructive ? ` **${t.destructive}**` : '';
      return `| ${cell('tiktools ' + verb.usage)} | ${local(verb)}${marker} |`;
    }).join('\n') + '\n';
  }
  text += `\n### ${t.conventions}\n\n`;
  text += help.conventions.map(({ topic, topics, text: body, texts }) =>
    `- **${topics?.[locale] ?? topic}**: ${texts?.[locale] ?? body}`).join('\n') + '\n';
  return text;
}

/**
 * Renders one domain's verb table for embedding inside a guide article: the
 * same data as the full reference, scoped so a feature guide can cross-link
 * its commands without duplicating the whole registry.
 */
export function renderCliDomainFragment(help: CliHelp, domain: string, locale: DocLocale): string {
  const t = labels[locale];
  const local = (entry: { summary?: string; summaries?: Partial<Record<DocLocale, string>> }) =>
    entry.summaries?.[locale] ?? entry.summary ?? '';
  const entry = help.commands.find((candidate) => candidate.domain === domain);
  if (!entry) throw new Error(`Unknown CLI domain in fragment request: ${domain}`);
  let text = `| ${t.command} | ${t.summary} |\n| --- | --- |\n`;
  text += entry.verbs.map((verb) => {
    const marker = verb.destructive ? ` **${t.destructive}**` : '';
    return `| ${cell('tiktools ' + verb.usage)} | ${local(verb)}${marker} |`;
  }).join('\n') + '\n';
  return text;
}
