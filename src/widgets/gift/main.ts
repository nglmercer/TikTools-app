/**
 * Gift Alert widget entry. Wires the shared runtime (credentials, gateway
 * client, reconnect) to the GiftController and renders alerts with Vue.
 */

import { createApp, h, ref } from 'vue';
import GiftWidget from './GiftWidget.vue';
import './gift.css';
import { parseDemoMode, parseGiftSettings } from '../shared/config.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import { GiftController, type GiftAlertView } from '../shared/gift-controller.ts';
import {
  makeTestFollowEnvelope,
  makeTestGiftCombo,
  makeTestGiftEnvelope,
  mountWidgetTestHook,
  type TestFollowOverrides,
  type TestGiftOverrides,
} from '../shared/test-events.ts';
import { createWidgetRuntime } from '../shared/widget-runtime.ts';

const settings = parseGiftSettings(window.location.search);
const demo = parseDemoMode(window.location.hash);
const alert = ref<GiftAlertView | null>(null);
const status = ref<GatewayStatus>('idle');
const debug = Boolean(import.meta.env.DEV);

const controller = new GiftController(
  {
    onShow: (next) => {
      alert.value = { ...next };
    },
    onUpdate: (next) => {
      alert.value = { ...next };
    },
    onHide: () => {
      alert.value = null;
    },
  },
  {
    visibleMs: settings.visibleMs,
    comboTimeoutMs: settings.comboTimeoutMs,
    minimumDiamonds: settings.minimumDiamonds,
  },
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
});

if (demo === 'gift') {
  window.setTimeout(() => runtime.injectTestEnvelope(makeTestGiftEnvelope()), 600);
} else if (demo === 'combo') {
  // Stagger the streak so previews show the count ticking up live.
  for (const [index, envelope] of makeTestGiftCombo(10).entries()) {
    window.setTimeout(() => runtime.injectTestEnvelope(envelope), 600 + index * 150);
  }
}

createApp({
  setup: () =>
    () =>
      h(GiftWidget, {
        alert: alert.value,
        status: status.value,
        settings,
        debug,
      }),
}).mount('#app');
