import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Subscribe Alert widget, served by the gateway at `/widgets/subscribe/`. */
export default createWidgetViteConfig(import.meta.dirname, 'subscribe');
