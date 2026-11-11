import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Cloud Overlay widget, served by the gateway at `/widgets/cloud-overlay/`. */
export default createWidgetViteConfig(import.meta.dirname, 'cloud-overlay');
