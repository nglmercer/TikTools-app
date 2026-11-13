/** Explicit editorial action; checks and builds never update review stamps. */
import { readFile, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { docLocales } from '../locales.ts';
import { articleBody, contentHash, contentRoot } from './content.ts';

const tails = process.argv.slice(2);
if (!tails.length) throw new Error('After reviewing all translations: bun docs/scripts/record-review.ts developer/example.md [user/example.md ...]');
for (const tail of tails) {
  const source = resolve(contentRoot, 'en', tail);
  if (!tail.endsWith('.md') || relative(resolve(contentRoot, 'en'), source).startsWith('..')) throw new Error(`Invalid article path: ${tail}`);
  const sourceRevision = contentHash(articleBody(await readFile(source, 'utf8')));
  for (const locale of docLocales) {
    const path = resolve(contentRoot, locale, tail);
    const text = await readFile(path, 'utf8');
    const translationRevision = contentHash(articleBody(text));
    await writeFile(path, text.replace(/^sourceRevision:.*$/m, `sourceRevision: "${sourceRevision}"`).replace(/^translationRevision:.*$/m, `translationRevision: "${translationRevision}"`));
  }
  console.log(`Recorded source and localized body reviews: ${tail}`);
}
