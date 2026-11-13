import { createHash } from 'node:crypto';
import { readdir, readFile } from 'node:fs/promises';
import { resolve, relative } from 'node:path';
import { createMarkdownRenderer } from 'vitepress';
import { docLocales, localizedDocumentPath } from '../locales.ts';

export const contentRoot = resolve(import.meta.dirname, '../content');
export const contentHash = (body: string) => createHash('sha256').update(body.trim()).digest('hex');
export function articleWordCount(body: string, locale: string): number {
  const prose = body.replace(/```[\s\S]*?```/g, '').replace(/<[^>]*>/g, '');
  return Array.from(new Intl.Segmenter(locale, { granularity: 'word' }).segment(prose)).filter((part) => part.isWordLike).length;
}
export const articleBody = (source: string) => source.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, '').trim();

export async function markdownFiles(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const groups = await Promise.all(entries.filter((entry) => !entry.name.startsWith('.')).map((entry) => entry.isDirectory() ? markdownFiles(resolve(directory, entry.name)) : entry.name.endsWith('.md') ? [resolve(directory, entry.name)] : []));
  return groups.flat().sort();
}

export interface Article {
  path: string;
  relativePath: string;
  source: string;
  body: string;
  frontmatter: Record<string, unknown>;
  headings: Array<{ level: string; id: string }>;
  links: Array<{ target: string; image: boolean }>;
}

export async function readArticles(paths?: readonly string[]): Promise<Article[]> {
  const md = await createMarkdownRenderer(contentRoot);
  const result: Article[] = [];
  for (const path of paths ?? await markdownFiles(contentRoot)) {
    const source = await readFile(path, 'utf8');
    const relativePath = relative(contentRoot, path).replaceAll('\\', '/');
    const env: { path: string; relativePath: string; content?: string; frontmatter?: Record<string, unknown> } = { path, relativePath };
    // VitePress 1.x extracts YAML in render(), not in MarkdownIt's parse().
    md.render(source, env);
    const tokens = md.parse(env.content ?? articleBody(source), env);
    const headings: Article['headings'] = [];
    const links: Article['links'] = [];
    for (const token of tokens) {
      if (token.type === 'heading_open') headings.push({ level: token.tag, id: token.attrGet('id') ?? '' });
      for (const child of token.children ?? []) {
        if (child.type === 'link_open') links.push({ target: child.attrGet('href') ?? '', image: false });
        if (child.type === 'image') links.push({ target: child.attrGet('src') ?? '', image: true });
      }
    }
    result.push({ path, relativePath, source, body: articleBody(source), frontmatter: env.frontmatter ?? {}, headings, links });
  }
  return result;
}

/** Project metadata is validated here; VitePress does not enforce these conventions. */
export function metadataErrors(articles: readonly Article[]): string[] {
  const errors: string[] = [];
  const seen = new Set<string>();
  const english = new Map(articles.filter((article) => article.relativePath.startsWith('en/')).map((article) => [String(article.frontmatter.translationKey), article]));
  for (const article of articles) {
    const { frontmatter: data, relativePath: path } = article;
    for (const key of ['title', 'docId', 'translationKey', 'locale', 'audience', 'featureStatus', 'contentStatus', 'sourceLocale', 'sourceRevision', 'translationRevision']) {
      if (typeof data[key] !== 'string' || !data[key]) errors.push(`${path}: missing ${key}`);
    }
    if (!(docLocales as readonly string[]).includes(String(data.locale))) errors.push(`${path}: invalid document locale`);
    if (!['user', 'developer', 'all'].includes(String(data.audience))) errors.push(`${path}: invalid audience`);
    if (!['available', 'experimental', 'planned', 'historical'].includes(String(data.featureStatus))) errors.push(`${path}: invalid feature status`);
    if (data.contentStatus !== 'reviewed') errors.push(`${path}: unreviewed public content`);
    if (data.sourceLocale !== 'en') errors.push(`${path}: source locale must be explicit English`);
    if (article.headings.filter((heading) => heading.level === 'h1').length !== 1) errors.push(`${path}: expected one main heading`);
    if (articleWordCount(article.body, docLocales.includes(data.locale as typeof docLocales[number]) ? String(data.locale) : 'en') < 40) errors.push(`${path}: insufficient article content`);
    if (/\bTODO\b|Coming soon|Próximamente/.test(article.body)) errors.push(`${path}: unfinished public content`);
    const identity = `${data.docId}:${data.locale}`;
    if (seen.has(identity)) errors.push(`${path}: duplicate document identity ${identity}`);
    seen.add(identity);
    const localized = localizedDocumentPath(path);
    if (!localized) continue; // The language root selector is the explicit x-default fallback.
    if (!path.startsWith(`${String(data.locale)}/`)) errors.push(`${path}: locale/path mismatch`);
    const source = english.get(String(data.translationKey));
    if (!source) { errors.push(`${path}: missing English source`); continue; }
    const revision = contentHash(source.body);
    if (data.sourceRevision !== revision || data.translationRevision !== contentHash(article.body)) errors.push(`${path}: source or translation revision is stale`);
    for (const locale of docLocales) {
      const counterpart = `${locale}/${localized.tail}`;
      if (!articles.some((entry) => entry.relativePath === counterpart && entry.frontmatter.translationKey === data.translationKey)) errors.push(`${path}: missing equivalent translation ${counterpart}`);
    }
    if (data.locale !== 'en' && article.body === source.body) errors.push(`${path}: untranslated English article`);
    if (data.locale !== 'en' && JSON.stringify(article.headings) !== JSON.stringify(source.headings)) errors.push(`${path}: translated heading anchors differ from the source`);
  }
  return errors;
}
