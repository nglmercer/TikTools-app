import { realpath } from 'node:fs/promises';
import { resolve, sep } from 'node:path';
import { siteConfig } from './lib/site-config.ts';

const root = await realpath(resolve(import.meta.dirname, '../dist/site'));
const { base } = siteConfig();
const port = Number(process.env.TIKTOOLS_SITE_PORT ?? 4173);
if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid site preview port.');
const server = Bun.serve({
  hostname: '127.0.0.1', port,
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === '/' && base !== '/') return Response.redirect(new URL(base, url), 302);
    if (!url.pathname.startsWith(base)) return new Response('Not found', { status: 404 });
    let path: string;
    try { path = decodeURIComponent(url.pathname.slice(base.length)); } catch { return new Response('Bad path', { status: 400 }); }
    if (path.endsWith('/') || path === '') path += 'index.html';
    const candidate = resolve(root, path);
    if (!candidate.startsWith(root + sep)) return new Response('Not found', { status: 404 });
    try {
      const actual = await realpath(candidate);
      if (!actual.startsWith(root + sep)) return new Response('Not found', { status: 404 });
      const file = Bun.file(actual);
      if (!await file.exists()) return new Response('Not found', { status: 404 });
      return new Response(file, { headers: { 'Cache-Control': 'no-store' } });
    } catch { return new Response('Not found', { status: 404 }); }
  },
});
console.log(`TikTools site preview: http://${server.hostname}:${server.port}${base}`);
