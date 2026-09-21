import { join, resolve } from 'node:path';

/**
 * Static server for the built OBS widgets. Serves `dist/widgets/` over
 * loopback for Playwright (`bun run test:widgets-e2e`) and manual checks.
 * Run `bun run build:widgets` first; the gateway serves these same bytes in
 * production.
 */

const repositoryRoot = resolve(import.meta.dir, '..');
const widgetsRoot = join(repositoryRoot, 'dist', 'widgets');

const MIME: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.wasm': 'application/wasm',
};

function portFromArgs(): number {
  const args = process.argv.slice(2);
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index] as string;
    if (arg === '--port' && args[index + 1]) return Number(args[index + 1]);
    if (arg.startsWith('--port=')) return Number(arg.slice('--port='.length));
  }
  const env = Number(process.env.WIDGETS_TEST_PORT ?? 18789);
  return Number.isInteger(env) && env >= 1 && env <= 65535 ? env : 18789;
}

const port = portFromArgs();

const followIndex = Bun.file(join(widgetsRoot, 'follow', 'index.html'));
const giftIndex = Bun.file(join(widgetsRoot, 'gift', 'index.html'));
if (!(await followIndex.exists()) || !(await giftIndex.exists())) {
  throw new Error(`Widget assets missing under ${widgetsRoot}. Run \`bun run build:widgets\` first.`);
}

Bun.serve({
  port,
  hostname: '127.0.0.1',
  async fetch(request) {
    const url = new URL(request.url);
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      return new Response('method not allowed', { status: 405 });
    }
    const segments = url.pathname.split('/').filter((segment) => segment !== '');
    if (segments.length === 0 || (segments[0] !== 'follow' && segments[0] !== 'gift')) {
      return new Response('not found', { status: 404 });
    }
    if (segments.some((segment) => segment === '.' || segment === '..' || segment.includes('\\'))) {
      return new Response('not found', { status: 404 });
    }
    const relative = segments.length === 1 ? [...segments, 'index.html'] : segments;
    const file = Bun.file(join(widgetsRoot, ...relative));
    if (!(await file.exists())) {
      return new Response('not found', { status: 404 });
    }
    const dot = relative[relative.length - 1]?.lastIndexOf('.') ?? -1;
    const extension = dot >= 0 ? (relative[relative.length - 1]?.slice(dot) ?? '') : '';
    return new Response(file, {
      headers: {
        'content-type': MIME[extension] ?? 'application/octet-stream',
        'cache-control': 'no-store',
      },
    });
  },
});

console.log(`Serving widget assets from ${widgetsRoot} at http://127.0.0.1:${port}/`);
