import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Top Likes widget, served by the gateway at `/widgets/likes/`. */
export default createWidgetViteConfig(import.meta.dirname, 'likes');
