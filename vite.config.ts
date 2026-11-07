import { defineConfig, type Plugin, type ProxyOptions } from 'vite';
import vue from '@vitejs/plugin-vue';
import { readFile } from 'node:fs/promises';
import { resolve, sep } from 'node:path';

const root = resolve(process.cwd(), 'src/web');

/**
 * Development-only plugin asset server: maps `/__plugins/<id>/...` to the
 * package's compiled `ui/dist/` output so isolated plugin UIs load in the
 * sandboxed dev iframe and Playwright. Packaged desktop builds never use
 * this — the native shell serves the same files over `tiktools-plugin://`
 * with its own sandbox. Traversal outside the package dist is rejected.
 *
 * Header parity with the production asset server
 * (`crates/tiktools-desktop/src/plugin_webview/assets.rs`) is intentional:
 * the dev iframe is sandboxed opaque-origin just like production, so every
 * response — success or error — carries `Access-Control-Allow-Origin: *`
 * and the framing policy. Error statuses must stay visible to the frame
 * console instead of hiding behind CORS failures.
 */
function pluginAssetServer(): Plugin {
  const packagesRoot = resolve(process.cwd(), 'plugins');
  // Mirrors the desktop `content_type()` map: module scripts and
  // stylesheets must never fall back to `application/octet-stream`.
  const mime: Record<string, string> = {
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
  // One framing policy: CSP `frame-ancestors` limited to loopback dev
  // servers. `X-Frame-Options: SAMEORIGIN` is deliberately not used — it is
  // redundant with `frame-ancestors` and its same-origin check is defined
  // against the framed URL origin, which is unreliable once the frame is
  // sandboxed to an opaque origin; `frame-ancestors` checks the embedder
  // instead and behaves consistently in every engine.
  const framing = 'frame-ancestors http://127.0.0.1:* http://localhost:*';
  return {
    name: 'tiktools-plugin-assets',
    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        const fail = (status: number, message: string): void => {
          response.statusCode = status;
          response.setHeader('content-type', 'text/plain; charset=utf-8');
          response.setHeader('access-control-allow-origin', '*');
          response.setHeader('content-security-policy', framing);
          response.end(message);
        };
        try {
          const url = new URL(request.url ?? '/', 'http://127.0.0.1');
          if (process.env.TIKTOOLS_DEV_WEB_ADAPTER) { next(); return; }
          if (!url.pathname.startsWith('/__plugins/')) {
            next();
            return;
          }
          const rest = url.pathname.slice('/__plugins/'.length);
          const slash = rest.indexOf('/');
          if (slash <= 0) {
            fail(404, 'missing plugin asset path');
            return;
          }
          const id = rest.slice(0, slash);
          const asset = rest.slice(slash + 1);
          if (!/^[a-z][a-z0-9._-]{1,127}$/.test(id) || !asset || asset.includes('\\')) {
            fail(400, 'invalid plugin asset path');
            return;
          }
          const file = resolve(packagesRoot, id, 'ui', 'dist', ...asset.split('/'));
          const dist = resolve(packagesRoot, id, 'ui', 'dist') + sep;
          if (!file.startsWith(dist)) {
            fail(403, 'asset escapes plugin dist');
            return;
          }
          const body = await readFile(file);
          const dot = file.lastIndexOf('.');
          const type = dot >= 0 ? (mime[file.slice(dot)] ?? 'application/octet-stream') : 'application/octet-stream';
          response.setHeader('content-type', type);
          response.setHeader('access-control-allow-origin', '*');
          response.setHeader('content-security-policy', framing);
          response.end(body);
        } catch {
          fail(404, 'plugin asset not found');
        }
      });
    },
  };
}

function localBrowserDevelopment(): Plugin {
  return {
    name: 'tiktools-local-browser-development',
    configureServer(server) {
      const adapter = process.env.TIKTOOLS_DEV_WEB_ADAPTER;
      if (!adapter) return;
      server.middlewares.use(async (request, response, next) => {
        const path = (request.url ?? '/').split('?')[0];
        const origin = `http://127.0.0.1:${process.env.TIKTOOLS_WEB_PORT}`;
        if ((path?.startsWith('/__local/') || path?.startsWith('/__plugins/')) && request.headers.origin && request.headers.origin !== origin && !(path?.startsWith('/__plugins/') && request.headers.origin === 'null')) {
          response.statusCode = 403; response.end('Forbidden'); return;
        }
        if (path !== '/' && path !== '/index.html') { next(); return; }
        try {
          const upstream = await fetch(`${adapter}/`, { headers: { ...(request.headers.cookie ? { cookie: request.headers.cookie } : {}) } });
          const html = await upstream.text();
          const meta = html.match(/<meta name="tiktools-transport"[^>]*><meta name="tiktools-plugin-assets"[^>]*>/)?.[0];
          response.setHeader('Content-Type', 'text/html; charset=utf-8');
          response.setHeader('Cache-Control', 'no-store');
          if (!meta) { response.end(html); return; }
          const index = await readFile(resolve(root, 'index.html'), 'utf8');
          response.end(await server.transformIndexHtml(request.url ?? '/', index.replace('<head>', `<head>${meta}`)));
        } catch { response.statusCode = 502; response.end('TikTools browser adapter unavailable'); }
      });
    },
  };
}

const adapter = process.env.TIKTOOLS_DEV_WEB_ADAPTER;
const devOrigin = `http://127.0.0.1:${process.env.TIKTOOLS_WEB_PORT}`;

export default defineConfig({
  root,
  plugins: [vue(), localBrowserDevelopment(), pluginAssetServer()],
  resolve: {
    alias: { '@': root },
  },
  define: {
    __VUE_I18N_FULL_INSTALL__: true,
    __VUE_I18N_LEGACY_API__: false,
    __INTLIFY_PROD_DEVTOOLS__: false,
  },
  build: {
    outDir: resolve(process.cwd(), 'dist/web'),
    emptyOutDir: true,
    sourcemap: true,
    rollupOptions: {
      output: {
        // Pin the Vue runtime to its own chunk so the tab
        // views split into per-tab files instead of one
        // monolith; every tab then shares the runtime chunk.
        manualChunks: (id) => {
          if (id.includes('node_modules/vue/') || id.includes('node_modules/@vue/')) {
            return 'vue';
          }
        },
      },
    },
  },
  server: {
    host: '127.0.0.1',
    port: Number(process.env.TIKTOOLS_WEB_PORT ?? 3000),
    // The dev launcher allocates the port and passes the actual URL to the
    // desktop via TIKTOOLS_DEV_URL. Never drift to another port silently.
    strictPort: true,
    ...(adapter ? { proxy: Object.fromEntries(['/__local', '/__plugins'].map((path) => [path, {
      target: adapter,
      changeOrigin: true,
      ws: true,
      configure: (proxy) => {
        proxy.on('proxyReq', (proxyRequest, request) => {
          if (request.headers.origin && request.headers.origin !== 'null') proxyRequest.setHeader('Origin', adapter);
        });
        proxy.on('proxyReqWs', (proxyRequest, request, socket) => {
          if (request.headers.origin !== devOrigin) { socket.destroy(); return; }
          proxyRequest.setHeader('Origin', adapter);
        });
      },
    } satisfies ProxyOptions])) } : {}),
  },
});
