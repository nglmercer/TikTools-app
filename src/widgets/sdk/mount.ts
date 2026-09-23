import { createApp, h } from 'vue';
import WidgetHost from './WidgetHost.vue';
import type { WidgetStyle, WidgetTemplate } from './template.ts';
import { parseDemoMode } from '../shared/config.ts';
import './standalone.css';
import { designFromHash } from './design.ts';
import { isWidgetTestHookEnabled } from './test-hook.ts';

/** Minimal OBS entry point. The editor uses WidgetHost directly. */
export function mountWidget(template: WidgetTemplate, options: { design?: WidgetStyle } = {}) {
  const app = createApp({
    render: () => h(WidgetHost, {
      template,
      design: options.design ?? designFromHash(window.location.hash),
      search: window.location.search,
      mode: parseDemoMode(window.location.hash) ? 'preview' : 'live',
      debug: Boolean(import.meta.env.DEV),
      testHook: isWidgetTestHookEnabled(),
    }),
  });
  app.mount('#app');
  const dispose = () => app.unmount();
  window.addEventListener('pagehide', dispose, { once: true });
  return { dispose };
}
