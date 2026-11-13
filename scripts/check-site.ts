import { readdir, readFile, access } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { docLocales } from '../docs/locales.ts';
import { siteConfig } from './lib/site-config.ts';
import { parseDemoOptions, validDemoRoute } from '../src/web/demo/scenarios.ts';
const root = resolve(import.meta.dirname, '../dist/site');
const site = siteConfig();
async function files(path: string): Promise<string[]> { return (await Promise.all((await readdir(path, { withFileTypes: true })).map((entry) => entry.isDirectory() ? files(resolve(path, entry.name)) : [resolve(path, entry.name)]))).flat(); }
const html = new Map(await Promise.all((await files(root)).filter((path) => path.endsWith('.html')).map(async (path) => [path, await readFile(path, 'utf8')] as const)));
const errors: string[] = [];
let links = 0;
for (const [path, source] of html) {
  const name = relative(root, path).replaceAll('\\', '/');
  if (name === '404.html' || name.startsWith('demo/')) continue;
  if ((source.match(/<h1(?:\s|>)/g) ?? []).length !== 1) errors.push(`${name}: expected exactly one rendered h1`);
  const expected = new URL(name.replace(/index\.html$/, ''), site.url).href;
  if (!source.includes(`rel="canonical" href="${expected}"`)) errors.push(`${name}: incorrect canonical`);
  for (const language of [...docLocales, 'x-default']) if (!source.includes(`hreflang="${language}"`)) errors.push(`${name}: missing hreflang ${language}`);
  for (const match of source.matchAll(/\b(?:href|src)="([^"]+)"/g)) {
    const value = match[1]!.replaceAll('&amp;', '&');
    if (/^(https?:|mailto:|data:|blob:)/.test(value)) continue;
    if (/^javascript:/i.test(value)) { errors.push(`${name}: executable link`); continue; }
    let url: URL;
    try { url = new URL(value, new URL(name, site.url)); } catch { errors.push(`${name}: invalid URL ${value}`); continue; }
    if (url.origin !== site.origin) continue;
    if (!url.pathname.startsWith(site.base)) { errors.push(`${name}: link escapes deployment base: ${value}`); continue; }
    let target = resolve(root, decodeURIComponent(url.pathname.slice(site.base.length)));
    if (url.pathname.endsWith('/')) target = resolve(target, 'index.html');
    if (relative(root, target).startsWith('..')) { errors.push(`${name}: path escapes artifact`); continue; }
    try { await access(target); } catch { errors.push(`${name}: missing built target ${value}`); continue; }
    if (relative(root, target).replaceAll('\\', '/') === 'demo/index.html') {
      try { parseDemoOptions(url.search); if (url.hash && !validDemoRoute(decodeURIComponent(url.hash.slice(1)))) throw new Error(); } catch { errors.push(`${name}: invalid built demo route ${value}`); }
    } else if (url.hash && html.has(target) && !html.get(target)!.includes(`id="${decodeURIComponent(url.hash.slice(1))}"`)) errors.push(`${name}: missing built anchor ${value}`);
    links++;
  }
}
if (errors.length) throw new Error(errors.join('\n'));
console.log(`Verified ${html.size} rendered HTML files and ${links} built links/assets/anchors under ${site.base}.`);
