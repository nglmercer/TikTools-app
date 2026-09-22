/**
 * Follow Alert widget entry. Wires the shared runtime (credentials, gateway
 * client, reconnect) to the FollowController and renders alerts with Vue.
 */

import { createApp, h, ref } from 'vue';
import FollowWidget from './FollowWidget.vue';
import './follow.css';
import { parseDemoMode, parseFollowSettings } from '../shared/config.ts';
import { FollowController, type FollowAlert } from '../shared/follow-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import {
  makeTestFollowEnvelope,
  makeTestGiftCombo,
  makeTestGiftEnvelope,
  mountWidgetTestHook,
  type TestFollowOverrides,
  type TestGiftOverrides,
} from '../shared/test-events.ts';
import { createWidgetRuntime } from '../shared/widget-runtime.ts';

const settings = parseFollowSettings(window.location.search);
const demo = parseDemoMode(window.location.hash);
const alert = ref<FollowAlert | null>(null);
const status = ref<GatewayStatus>('idle');
const debug = Boolean(import.meta.env.DEV);

const controller = new FollowController(
  {
    onShow: (next) => {
      alert.value = next;
    },
    onHide: () => {
      alert.value = null;
    },
  },
  { visibleMs: settings.visibleMs },
);

const runtime = createWidgetRuntime({
  onEnvelope: (envelope) => controller.handleEnvelope(envelope),
  onStatus: (next) => {
    status.value = next;
  },
});
runtime.start();

mountWidgetTestHook({
  emitEnvelope: (envelope) => runtime.injectTestEnvelope(envelope),
  emitTestFollow: (overrides?: TestFollowOverrides) =>
    runtime.injectTestEnvelope(makeTestFollowEnvelope(overrides)),
  emitTestGift: (overrides?: TestGiftOverrides) =>
    runtime.injectTestEnvelope(makeTestGiftEnvelope(overrides)),
  emitTestGiftCombo: (count: number, overrides?: TestGiftOverrides) => {
    for (const envelope of makeTestGiftCombo(count, overrides)) {
      runtime.injectTestEnvelope(envelope);
    }
  },
  connectionStatus: () => runtime.status,
});

if (demo === 'follow') {
  window.setTimeout(() => runtime.injectTestEnvelope(makeTestFollowEnvelope()), 600);
}

createApp({
  setup: () =>
    () =>
      h(FollowWidget, {
        alert: alert.value,
        status: status.value,
        settings,
        debug,
      }),
}).mount('#app');
