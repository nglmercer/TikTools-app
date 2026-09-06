import { createApp } from 'vue';
import App from './App.vue';
import { installControlEventBridge } from './components/ui/control-events.ts';
import './styles.css';

declare global {
  interface Window {
    ipc?: { postMessage: (message: string) => void };
  }
}

installControlEventBridge();
const app = createApp(App);
app.mount('#app');
// The native window remains hidden until Vue has mounted successfully. This
// host-only signal is deliberately separate from the normal page IPC model.
// A missing bridge is a startup bug, not a state to ignore: fail loudly so
// the desktop host logs a page-load transition instead of timing out after
// 10 seconds with no frontend diagnostic.
if (!window.ipc) {
  throw new Error('TikTools native IPC bridge is unavailable during frontend startup');
}

window.ipc.postMessage(JSON.stringify({ type: 'frontend-ready' }));
