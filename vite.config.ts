import { defineConfig, type Plugin, type ProxyOptions } from 'vite';
import vue from '@vitejs/plugin-vue';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const root = resolve(process.cwd(), 'src/web');

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
  plugins: [vue(), localBrowserDevelopment()],
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
