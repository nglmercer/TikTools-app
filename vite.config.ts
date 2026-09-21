import { defineConfig, type Plugin } from 'vite';
import vue from '@vitejs/plugin-vue';
import vueJsx from '@vitejs/plugin-vue-jsx';
import { readFile } from 'node:fs/promises';
import { resolve, sep } from 'node:path';

const root = resolve(process.cwd(), 'src/web');

/**
 * Development-only plugin asset server: maps `/__plugins/<id>/...` to the
 * package's compiled `ui/dist/` output so isolated plugin UIs load in the
 * sandboxed dev iframe and Playwright. Packaged desktop builds never use
 * this — the native shell serves the same files over `tiktools-plugin://`
 * with its own sandbox. Traversal outside the package dist is rejected.
 */
function pluginAssetServer(): Plugin {
  const packagesRoot = resolve(process.cwd(), 'plugins');
  const mime: Record<string, string> = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json; charset=utf-8',
    '.svg': 'image/svg+xml',
    '.png': 'image/png',
    '.woff2': 'font/woff2',
  };
  return {
    name: 'tiktools-plugin-assets',
    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        try {
          const url = new URL(request.url ?? '/', 'http://127.0.0.1');
          if (!url.pathname.startsWith('/__plugins/')) {
            next();
            return;
          }
          const rest = url.pathname.slice('/__plugins/'.length);
          const slash = rest.indexOf('/');
          if (slash <= 0) {
            response.statusCode = 404;
            response.end('missing plugin asset path');
            return;
          }
          const id = rest.slice(0, slash);
          const asset = rest.slice(slash + 1);
          if (!/^[a-z][a-z0-9._-]{1,127}$/.test(id) || !asset || asset.includes('\\')) {
            response.statusCode = 400;
            response.end('invalid plugin asset path');
            return;
          }
          const file = resolve(packagesRoot, id, 'ui', 'dist', ...asset.split('/'));
          const dist = resolve(packagesRoot, id, 'ui', 'dist') + sep;
          if (!file.startsWith(dist)) {
            response.statusCode = 403;
            response.end('asset escapes plugin dist');
            return;
          }
          const body = await readFile(file);
          const dot = file.lastIndexOf('.');
          const type = dot >= 0 ? (mime[file.slice(dot)] ?? 'application/octet-stream') : 'application/octet-stream';
          response.setHeader('content-type', type);
          // The dev iframe is sandboxed opaque-origin; keep framing local.
          response.setHeader('x-frame-options', 'SAMEORIGIN');
          response.end(body);
        } catch {
          response.statusCode = 404;
          response.end('plugin asset not found');
        }
      });
    },
  };
}

export default defineConfig({
  root,
  plugins: [vue(), vueJsx(), pluginAssetServer()],
  build: {
    outDir: resolve(process.cwd(), 'dist/web'),
    emptyOutDir: true,
    sourcemap: true,
  },
  server: {
    host: '127.0.0.1',
    port: Number(process.env.TIKTOOLS_WEB_PORT ?? 3000),
    // The dev launcher allocates the port and passes the actual URL to the
    // desktop via TIKTOOLS_DEV_URL. Never drift to another port silently.
    strictPort: true,
  },
});
