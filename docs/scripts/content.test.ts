import { describe, expect, test } from 'bun:test';
import { contentHash, metadataErrors, readArticles, type Article } from './content.ts';

const en = '# Install TikTools\n\n' + 'Read the release notes and choose the correct package for your operating system. '.repeat(4);
const es = '# Instalar TikTools\n\n' + 'Lee las notas de la versión y elige el paquete correspondiente a tu sistema operativo. '.repeat(4);
function article(locale: 'en' | 'es', body: string): Article {
  const relativePath = `${locale}/user/install.md`;
  return { path: relativePath, relativePath, source: body, body, headings: [{ level: 'h1', id: 'install' }], links: [], frontmatter: { title: 'Install', docId: 'user.install', translationKey: 'user.install', locale, audience: 'user', featureStatus: 'available', contentStatus: 'reviewed', sourceLocale: 'en', sourceRevision: contentHash(en), translationRevision: contentHash(en) } };
}

describe('public documentation metadata', () => {
  test('reads metadata through the installed VitePress frontmatter renderer', async () => {
    const articles = await readArticles();
    expect(articles.find((entry) => entry.relativePath === 'en/user/installation.md')?.frontmatter.docId).toBe('user.start.installation');
  });
  test('accepts a complete pair with an explicitly reviewed source revision', () => {
    expect(metadataErrors([article('en', en), article('es', es)])).toEqual([]);
  });
  test('rejects an absent equivalent-language article', () => {
    expect(metadataErrors([article('en', en)]).join('\n')).toContain('missing equivalent translation');
  });
  test('detects a translation left behind after English prose changes', () => {
    expect(metadataErrors([article('en', en + ' Updated instructions.'), article('es', es)]).join('\n')).toContain('revision is stale');
  });
  test('does not treat identical English prose as a Spanish translation', () => {
    expect(metadataErrors([article('en', en), article('es', en)]).join('\n')).toContain('untranslated English article');
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
