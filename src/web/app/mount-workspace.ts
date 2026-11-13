import { createApp, type App as VueApp } from 'vue';
import { createPinia, disposePinia } from 'pinia';
import type { ControlClient } from '@/shared/api/control-client.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { activateLocale, i18n, type Locale } from '@/shared/i18n/i18n.ts';
import App from './App.vue';
import { createAppRouter } from './router/router.ts';

/** Explicit shared assembly only; importing this module never mounts an app or selects a transport. */
export async function mountWorkspace(options: {
  client: ControlClient;
  locale: Locale;
  target?: string | Element;
  configure?(app: VueApp): void;
}): Promise<VueApp> {
  const pinia = createPinia();
  const router = createAppRouter();
  const app = createApp(App).use(pinia).use(i18n).use(router).provide(controlClientKey, options.client);
  options.configure?.(app);
  app.onUnmount(() => { disposePinia(pinia); router.options.history.destroy(); });
  try {
    await Promise.all([activateLocale(options.locale), router.isReady()]);
    app.mount(options.target ?? '#app');
  } catch (failure) {
    disposePinia(pinia); router.options.history.destroy(); options.client.detach();
    throw failure;
  }
  return app;
}
