import { createBrowserTransport } from '@/shared/api/browser-transport.ts';
import { createRemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { createBrowserControllerClient } from '@/features/controller/model/browser-controller.ts';
import { createPinia } from 'pinia';
import { createApp, type App as VueApp } from 'vue';
import { createControlClient, notifyFrontendReady } from '@/shared/api/control-client.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { activateLocale, i18n } from '@/shared/i18n/i18n.ts';
import { detectLocale } from '@/shared/i18n/locale-preference.ts';
import App from './App.vue';
import ControllerRoot from '@/features/controller/ui/ControllerRoot.vue';
import { createAppRouter } from './router/router.ts';
import './styles/tokens.css';
import './styles/base.css';
import './styles/local-browser.css';
import './styles/responsive.css';

declare global {
  interface Window {
    __tiktools_controller__?: { profile?: string | null };
    __tiktools_remote_client_available__?: boolean;
    __tiktools_remount__?: () => Promise<void>;
  }
}

async function mountApp(): Promise<VueApp> {
  const transportMode = document.querySelector('meta[name="tiktools-transport"]')?.getAttribute('content');
  if (transportMode === 'remote-web') {
    const transport = createRemoteBrowserTransport();
    const remote = createControlClient({ transport });
    const control = createBrowserControllerClient(remote, transport, location.origin);
    const app = createApp(ControllerRoot, { browserMode: true }).use(i18n).provide(controlClientKey, control);
    await activateLocale(detectLocale());
    app.mount('#app');
    return app;
  }
  if (window.__tiktools_controller__) {
    const app = createApp(ControllerRoot).use(i18n)
      .provide(controlClientKey, createControlClient());
    await activateLocale(detectLocale());
    app.mount('#app');
    notifyFrontendReady();
    return app;
  }
  const browser = transportMode === 'local-web';
  if (browser) document.documentElement.dataset['tiktoolsTransport'] = 'local-web';
  const banner = browser ? document.createElement('div') : null;
  if (banner) {
    banner.setAttribute('role', 'status');
    banner.style.cssText = 'position:fixed;bottom:0;left:0;right:0;z-index:9999;padding:6px;background:#202530;color:white;text-align:center;font-size:12px';
    document.body.append(banner);
  }
  const transport = browser ? createBrowserTransport({
    url: `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/__local/ws`,
    status: (message) => { if (banner) { banner.textContent = message; banner.hidden = message === 'Connected'; } },
  }) : undefined;
  if (transport) await transport.connect().catch(() => {});
  const router = createAppRouter();
  const app = createApp(App)
    .use(createPinia())
    .use(i18n)
    .use(router)
    .provide(controlClientKey, createControlClient({ transport }));
  app.onUnmount(() => banner?.remove());
  await Promise.all([activateLocale(detectLocale()), router.isReady()]);
  app.mount('#app');
  // The desktop WebView reveals its hidden window on frontend-ready. A browser page
  // has no native window and no window.ipc, so it must not send it.
  if (!browser) notifyFrontendReady();
  return app;
}

let app = await mountApp();

if (import.meta.env.DEV || import.meta.env.VITE_TIKTOOLS_DIAGNOSTICS === '1') {
  window.__tiktools_remount__ = async () => {
    app.unmount();
    app = await mountApp();
  };
}
