import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Gift Alert widget, served by the gateway at `/widgets/gift/`. */
export default createWidgetViteConfig(import.meta.dirname, 'gift');
