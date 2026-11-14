import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { readArticles } from './content.ts';
const links = new Map<string, string[]>();
for (const page of await readArticles()) for (const link of page.links) if (/^https?:/.test(link.target)) {
  const url = new URL(link.target); url.hash = '';
  links.set(url.href, [...(links.get(url.href) ?? []), page.relativePath]);
}
const queue = [...links];
const results: Array<{ url: string; status: number | null; state: 'available' | 'unreachable'; pages: string[] }> = [];
await Promise.all(Array.from({ length: 4 }, async () => {
  while (queue.length) {
    const [url, pages] = queue.shift()!;
    let status: number | null = null;
    try { const response = await fetch(url, { method: 'HEAD', redirect: 'follow', signal: AbortSignal.timeout(15000) }); status = response.status; if (status === 405) { const fallback = await fetch(url, { redirect: 'follow', signal: AbortSignal.timeout(15000) }); status = fallback.status; await fallback.body?.cancel(); } }
    catch { /* The report preserves unreachable endpoints for an explicit follow-up. */ }
    results.push({ url, status, state: status !== null && status >= 200 && status < 400 ? 'available' : 'unreachable', pages: [...new Set(pages)] });
  }
}));
results.sort((a, b) => a.url.localeCompare(b.url));
const reportPath = resolve(import.meta.dirname, '../../dist/reports/external-link-audit.json');
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, JSON.stringify({ reviewedAt: new Date().toISOString(), scope: 'Current public Markdown links. Immutable archive links remain inventoried as historical evidence.', results }, null, 2) + '\n');
console.log(`Report: ${reportPath}`);
console.log(`${results.length} public external destinations checked; ${results.filter((result) => result.state !== 'available').length} require review.`);
for (const row of results.filter((row) => row.state !== 'available')) console.log(`${row.status ?? 'network'} ${row.url}`);
if (results.some((row) => row.state !== 'available')) process.exitCode = 1;
