import { describe, expect, test } from 'bun:test';
import { docLocales, type DocLocale } from '../locales.ts';
import { articleWordCount, contentHash, metadataErrors, readArticles, type Article } from './content.ts';

const en = '# Install TikTools\n\n' + 'Read the release notes and choose the correct package for your operating system. '.repeat(4);
const es = '# Instalar TikTools\n\n' + 'Lee las notas de la versión y elige el paquete correspondiente a tu sistema operativo. '.repeat(4);
function article(locale: DocLocale, body: string): Article {
  const relativePath = `${locale}/user/install.md`;
  return { path: relativePath, relativePath, source: body, body, headings: [{ level: 'h1', id: 'install' }], links: [], frontmatter: { title: 'Install', docId: 'user.install', translationKey: 'user.install', locale, audience: 'user', featureStatus: 'available', contentStatus: 'reviewed', sourceLocale: 'en', sourceRevision: contentHash(en), translationRevision: contentHash(en) } };
}

const translations = () => [article('en', en), article('es', es), article('pt', '# Instalar TikTools\n\n' + 'Leia as notas da versão e escolha o pacote correto para o seu sistema operacional. '.repeat(4)), article('fr', '# Installer TikTools\n\n' + 'Lisez les notes de version et choisissez le paquet adapté à votre système d’exploitation. '.repeat(4)), article('zh', '# 安装 TikTools\n\n' + '请阅读版本说明，并为您的操作系统选择正确的软件包。'.repeat(8)), article('ja', '# TikTools をインストールする\n\n' + 'リリースノートを読み、使用するオペレーティングシステムに合ったパッケージを選択してください。'.repeat(8))];

describe('public documentation metadata', () => {
  test('reads metadata through the installed VitePress frontmatter renderer', async () => {
    const articles = await readArticles();
    expect(articles.find((entry) => entry.relativePath === 'en/user/installation.md')?.frontmatter.docId).toBe('user.start.installation');
  });
  test('accepts all six translations with an explicitly reviewed source revision', () => {
    expect(metadataErrors(translations())).toEqual([]);
  });
  test('counts meaningful CJK text without requiring inserted whitespace', () => {
    const zh = translations().find((article) => article.frontmatter.locale === 'zh')!;
    const ja = translations().find((article) => article.frontmatter.locale === 'ja')!;
    expect(articleWordCount(zh.body, 'zh')).toBeGreaterThan(40);
    expect(articleWordCount(ja.body, 'ja')).toBeGreaterThan(40);
    expect(articleWordCount('空', 'zh')).toBeLessThan(40);
  });
  test('rejects an absent equivalent-language article', () => {
    expect(metadataErrors([article('en', en)]).join('\n')).toContain('missing equivalent translation');
  });
  test('requires Portuguese and French counterparts rather than silently falling back', () => {
    const errors = metadataErrors(translations().filter((item) => !['pt', 'fr'].includes(String(item.frontmatter.locale)))).join('\n');
    expect(errors).toContain('pt/user/install.md');
    expect(errors).toContain('fr/user/install.md');
  });
  for (const locale of docLocales.filter((locale) => locale !== 'en')) test(`rejects untranslated or stale ${locale} prose`, () => {
    const all = translations();
    const localized = all.find((item) => item.frontmatter.locale === locale)!;
    localized.body = en;
    localized.frontmatter.translationRevision = 'stale';
    const errors = metadataErrors(all).join('\n');
    expect(errors).toContain('untranslated English article');
    expect(errors).toContain('revision is stale');
  });
  test('requires Chinese and Japanese editions without substituting a fallback', () => {
    const errors = metadataErrors(translations().filter((article) => !['zh', 'ja'].includes(String(article.frontmatter.locale)))).join('\n');
    expect(errors).toContain('zh/user/install.md');
    expect(errors).toContain('ja/user/install.md');
  });
  test('detects a translation left behind after English prose changes', () => {
    expect(metadataErrors([article('en', en + ' Updated instructions.'), article('es', es)]).join('\n')).toContain('revision is stale');
  });
  test('does not treat identical English prose as a Spanish translation', () => {
    expect(metadataErrors([article('en', en), article('es', en)]).join('\n')).toContain('untranslated English article');
  });
  test('preserves equivalent heading fragments when switching languages', () => {
    const translated = article('es', es);
    translated.headings[0]!.id = 'instalar';
    expect(metadataErrors([article('en', en), translated]).join('\n')).toContain('heading anchors differ');
  });
  test('rejects several main headings and draft public content', () => {
    const draft = article('es', es);
    draft.frontmatter.contentStatus = 'draft';
    draft.headings.push({ level: 'h1', id: 'other' });
    const errors = metadataErrors([article('en', en), draft]).join('\n');
    expect(errors).toContain('unreviewed public content');
    expect(errors).toContain('one main heading');
  });
});
