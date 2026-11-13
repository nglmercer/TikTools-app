import { createHash } from 'node:crypto';
import { readdir, readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, relative, dirname } from 'node:path';
import { createMarkdownRenderer } from 'vitepress';
import GithubSlugger from 'github-slugger';

const root = resolve(import.meta.dirname, '../..');
const docs = resolve(root, 'docs');
const baseline = resolve(docs, 'inventory.json');
const hash = (data: Uint8Array) => createHash('sha256').update(data).digest('hex');

async function files(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const groups = await Promise.all(entries.filter((entry) => !['node_modules', '.vitepress', 'content', 'internal'].includes(entry.name)).map((entry) => entry.isDirectory() ? files(resolve(directory, entry.name)) : [resolve(directory, entry.name)]));
  return groups.flat();
}

if (await Bun.file(baseline).exists()) throw new Error('The provenance inventory already exists; never overwrite the migration baseline.');
const md = await createMarkdownRenderer(root);
const sourceFiles = [...await files(docs), resolve(root, 'README.md'), resolve(root, 'CONTRIBUTING.md')];
const documents = [];
const assets = [];
for (const path of sourceFiles.sort()) {
  const bytes = await readFile(path);
  const source = relative(root, path).replaceAll('\\', '/');
  if (!path.endsWith('.md')) {
    if (/\.(png|jpe?g|svg|gif|webp|mmd|drawio)$/i.test(path)) assets.push({ source, sha256: hash(bytes), bytes: bytes.length });
    continue;
  }
  const tokens = md.parse(bytes.toString('utf8'), {});
  const slugger = new GithubSlugger();
  const anchors: string[] = [];
  const links: Array<{ target: string; kind: 'link' | 'image' }> = [];
  for (let index = 0; index < tokens.length; index++) {
    if (tokens[index]?.type === 'heading_open') {
      const inline = tokens[index + 1];
      const text = (inline?.children ?? []).filter((child) => child.type !== 'html_inline').map((child) => child.content).join('');
      anchors.push(slugger.slug(text));
    }
    for (const child of tokens[index]?.children ?? []) {
      if (child.type === 'link_open') links.push({ target: child.attrGet('href') ?? '', kind: 'link' });
      if (child.type === 'image') links.push({ target: child.attrGet('src') ?? '', kind: 'image' });
    }
  }
  documents.push({ source, sha256: hash(bytes), bytes: bytes.length, anchors, links });
}
await mkdir(dirname(baseline), { recursive: true });
await writeFile(baseline, `${JSON.stringify({ version: 1, sourceCommit: '80c5cb1fbfa45eaeb1c086c71320a15e565d8541', reviewedFromCommit: 'b72370014ac0cdd4da01be43e7ad86d19d4c9d1c', documents, assets }, null, 2)}\n`);
console.log(`Inventoried ${documents.length} Markdown documents and ${assets.length} images/diagrams.`);
