/**
 * Chat Overlay widget entry. Wires the shared runtime (credentials, gateway
 * client, reconnect) to the ChatController and renders messages with Vue.
 */

import { createApp, h, ref } from 'vue';
import ChatWidget from './ChatWidget.vue';
import './chat.css';
import { parseChatSettings, parseDemoMode } from '../shared/config.ts';
import { ChatController, type ChatMessageView } from '../shared/chat-controller.ts';
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

const settings = parseChatSettings(window.location.search);
const demo = parseDemoMode(window.location.hash);
const messages = ref<ChatMessageView[]>([]);
const status = ref<GatewayStatus>('idle');
const debug = Boolean(import.meta.env.DEV);

const controller = new ChatController(
  {
    onMessages: (next) => {
      messages.value = next;
    },
  },
  { limit: settings.limit },
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

if (demo === 'chat') {
  const samples: TestChatOverrides[] = [
    { comment: 'Hello everyone, first time catching the stream!', user: { nickname: 'Viewer One', uniqueId: 'viewer_one' } },
    { comment: 'That last gift combo was insane', user: { nickname: 'Gift Fan', uniqueId: 'gift_fan' } },
    { comment: 'Shoutout from the late crew', user: { nickname: 'Night Owl', uniqueId: 'night_owl' } },
    { comment: 'Lets gooo, hype in the chat!', user: { nickname: 'Hype Train', uniqueId: 'hype_train' } },
  ];
  for (const [index, overrides] of samples.entries()) {
    window.setTimeout(() => runtime.injectTestEnvelope(makeTestChatEnvelope(overrides)), 600 + index * 700);
  }
}

createApp({
  setup: () =>
    () =>
      h(ChatWidget, {
        messages: messages.value,
        status: status.value,
        settings,
        debug,
      }),
}).mount('#app');
