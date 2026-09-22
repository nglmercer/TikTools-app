/**
 * Share Alert widget entry. Wires the shared runtime (credentials, gateway
 * client, reconnect) to the ShareController and renders alerts with Vue.
 */

import { createApp, h, ref } from 'vue';
import ShareWidget from './ShareWidget.vue';
import './share.css';
import { parseDemoMode, parseShareSettings } from '../shared/config.ts';
import { ShareController, type ShareAlert } from '../shared/share-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import {
  makeTestChatEnvelope,
  makeTestFollowEnvelope,
  makeTestGiftCombo,
  makeTestGiftEnvelope,
  makeTestShareEnvelope,
  makeTestSubscribeEnvelope,
  mountWidgetTestHook,
  type TestChatOverrides,
  type TestFollowOverrides,
  type TestGiftOverrides,
  type TestShareOverrides,
  type TestSubscribeOverrides,
} from '../shared/test-events.ts';
import { createWidgetRuntime } from '../shared/widget-runtime.ts';

const settings = parseShareSettings(window.location.search);
const demo = parseDemoMode(window.location.hash);
const alert = ref<ShareAlert | null>(null);
const status = ref<GatewayStatus>('idle');
const debug = Boolean(import.meta.env.DEV);

const controller = new ShareController(
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
  emitTestChat: (overrides?: TestChatOverrides) =>
    runtime.injectTestEnvelope(makeTestChatEnvelope(overrides)),
  emitTestShare: (overrides?: TestShareOverrides) =>
    runtime.injectTestEnvelope(makeTestShareEnvelope(overrides)),
  emitTestSubscribe: (overrides?: TestSubscribeOverrides) =>
    runtime.injectTestEnvelope(makeTestSubscribeEnvelope(overrides)),
  connectionStatus: () => runtime.status,
});

if (demo === 'share') {
  window.setTimeout(() => runtime.injectTestEnvelope(makeTestShareEnvelope()), 600);
}

createApp({
  setup: () =>
    () =>
      h(ShareWidget, {
        alert: alert.value,
        status: status.value,
        settings,
        debug,
      }),
}).mount('#app');
