import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Share Alert widget, served by the gateway at `/widgets/share/`. */
export default createWidgetViteConfig(import.meta.dirname, 'share');
