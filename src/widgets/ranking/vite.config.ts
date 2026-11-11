import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Ranking widget, served by the gateway at `/widgets/ranking/`. */
export default createWidgetViteConfig(import.meta.dirname, 'ranking');
