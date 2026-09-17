import { createApp } from 'vue';
import App from './App.vue';
import { installControlEventBridge } from './components/ui/control-events.ts';
import { notifyFrontendReady } from './platform/control-client.ts';
import './styles.css';

installControlEventBridge();
const app = createApp(App);
app.mount('#app');
// The native window remains hidden until Vue has mounted successfully. This
// host-only signal is deliberately separate from the normal page IPC model.
// A missing bridge is a startup bug, not a state to ignore: fail loudly so
// the desktop host logs a page-load transition instead of timing out after
// 10 seconds with no frontend diagnostic.
notifyFrontendReady();
