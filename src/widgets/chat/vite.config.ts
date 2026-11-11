import { createWidgetViteConfig } from '../create-widget-vite-config.ts';

/** Standalone build of the Chat Overlay widget, served by the gateway at `/widgets/chat/`. */
export default createWidgetViteConfig(import.meta.dirname, 'chat');
