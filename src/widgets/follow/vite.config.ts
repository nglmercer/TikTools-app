import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Follow Alert widget, served by the gateway at `/widgets/follow/`. */
export default createWidgetViteConfig(import.meta.dirname, 'follow');
