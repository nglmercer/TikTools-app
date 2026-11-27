import { createBrowserTransport, LocalSessionError, touchLocalSession } from '@/shared/api/browser-transport.ts';
import { createSessionRecovery, sessionRecoveryKey, type SessionRecovery } from '@/shared/api/session-recovery.ts';
import type { App as VueApp } from 'vue';
import { createControlClient, notifyFrontendReady } from '@/shared/api/control-client.ts';
import { detectLocale } from '@/shared/i18n/locale-preference.ts';
import { mountWorkspace } from './mount-workspace.ts';
import './styles/tokens.css';
import './styles/base.css';
import './styles/local-browser.css';
import './styles/responsive.css';
import './styles/motion.css';

declare global {
  interface Window {
    __tiktools_remount__?: () => Promise<void>;
  }
}

async function mountApp(): Promise<VueApp> {
  const transportMode = document.querySelector('meta[name="tiktools-transport"]')?.getAttribute('content');
  const browser = transportMode === 'local-web';
  if (browser) document.documentElement.dataset['tiktoolsTransport'] = 'local-web';
  const banner = browser ? document.createElement('div') : null;
  if (banner) {
    banner.setAttribute('role', 'status');
    banner.style.cssText = 'position:fixed;bottom:0;left:0;right:0;z-index:9999;padding:6px;background:#202530;color:white;text-align:center;font-size:12px';
    document.body.append(banner);
  }
  let session: SessionRecovery | undefined;
  const transport = browser ? createBrowserTransport({
    url: `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/__local/ws`,
    autoReconnect: false,
    session: { probe: touchLocalSession },
    sessionState: (state) => session?.report(state),
    status: (message) => { if (banner) { banner.textContent = message; banner.hidden = message === 'Connected'; } },
  }) : undefined;
  if (browser) {
    session = createSessionRecovery({
      // The adapter answers any unauthenticated navigation with its pair page.
      pairUrl: '/',
      retry: async () => {
        try {
          await transport!.connect();
          return 'ok';
        } catch (error) {
          return error instanceof LocalSessionError ? error.kind : 'unavailable';
        }
      },
    });
  }
  if (transport) await transport.connect().catch(() => {});
  const app = await mountWorkspace({
    client: createControlClient({ transport }),
    locale: detectLocale(),
    configure(app) {
      if (session) app.provide(sessionRecoveryKey, session);
      app.onUnmount(() => banner?.remove());
      if (browser) {
        // Keep the sliding window alive while the page is open but idle, and
        // revalidate immediately when a hidden tab becomes visible again.
        const touch = (): void => {
          if (document.visibilityState === 'hidden') return;
          void touchLocalSession().then((state) => { if (state !== 'ok') session?.report(state); });
        };
        const timer = setInterval(touch, 5 * 60_000);
        document.addEventListener('visibilitychange', touch);
        app.onUnmount(() => { clearInterval(timer); document.removeEventListener('visibilitychange', touch); });
      }
    },
  });
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
