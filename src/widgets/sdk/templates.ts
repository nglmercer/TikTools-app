import { h, shallowRef } from 'vue';
import { defineWidget } from './template.ts';
import FollowWidget from '../follow/FollowWidget.vue';
import { FollowController, type FollowAlert } from '../shared/follow-controller.ts';
import ShareWidget from '../share/ShareWidget.vue';
import { ShareController, type ShareAlert } from '../shared/share-controller.ts';
import SubscribeWidget from '../subscribe/SubscribeWidget.vue';
import { SubscribeController, type SubscribeAlert } from '../shared/subscribe-controller.ts';
import GiftWidget from '../gift/GiftWidget.vue';
import ChatWidget from '../chat/ChatWidget.vue';
import { GiftController, type GiftAlertView } from '../shared/gift-controller.ts';
import { ChatController, type ChatMessageView } from '../shared/chat-controller.ts';
import { parseFollowSettings, parseShareSettings, parseSubscribeSettings, parseGiftSettings, parseChatSettings } from '../shared/config.ts';
import { makeTestFollowEnvelope, makeTestShareEnvelope, makeTestSubscribeEnvelope, makeTestGiftCombo, makeTestChatEnvelope } from '../shared/test-events.ts';

export const followTemplate = defineWidget({
  id: 'follow',
  samples: () => [makeTestFollowEnvelope()],
  create(search, clock) {
    const settings = parseFollowSettings(search);
    const alert = shallowRef<FollowAlert | null>(null);
    const controller = new FollowController({
      onShow: (next) => { alert.value = { ...next }; },
      onHide: () => { alert.value = null; },
    }, { visibleMs: settings.visibleMs, clock });
    return { controller, render: (status, debug) => h(FollowWidget, { alert: alert.value, settings, status, debug }) };
  },
});

export const shareTemplate = defineWidget({
  id: 'share',
  samples: () => [makeTestShareEnvelope()],
  create(search, clock) {
    const settings = parseShareSettings(search);
    const alert = shallowRef<ShareAlert | null>(null);
    const controller = new ShareController({
      onShow: (next) => { alert.value = { ...next }; },
      onHide: () => { alert.value = null; },
    }, { visibleMs: settings.visibleMs, clock });
    return { controller, render: (status, debug) => h(ShareWidget, { alert: alert.value, settings, status, debug }) };
  },
});

export const subscribeTemplate = defineWidget({
  id: 'subscribe',
  samples: () => [makeTestSubscribeEnvelope()],
  create(search, clock) {
    const settings = parseSubscribeSettings(search);
    const alert = shallowRef<SubscribeAlert | null>(null);
    const controller = new SubscribeController({
      onShow: (next) => { alert.value = { ...next }; },
      onHide: () => { alert.value = null; },
    }, { visibleMs: settings.visibleMs, clock });
    return { controller, render: (status, debug) => h(SubscribeWidget, { alert: alert.value, settings, status, debug }) };
  },
});

export const giftTemplate = defineWidget({
  id: 'gift',
  samples: () => makeTestGiftCombo(5),
  create(search, clock) {
    const settings = parseGiftSettings(search);
    const alert = shallowRef<GiftAlertView | null>(null);
    const update = (next: GiftAlertView) => { alert.value = { ...next }; };
    const controller = new GiftController({ onShow: update, onUpdate: update, onHide: () => { alert.value = null; } }, { ...settings, clock });
    return { controller, render: (status, debug) => h(GiftWidget, { alert: alert.value, settings, status, debug }) };
  },
});

export const chatTemplate = defineWidget({
  id: 'chat',
  samples: () => ['Hello everyone!', 'Love this LIVE!', 'Let’s go!'].map((comment) => makeTestChatEnvelope({ comment })),
  create(search) {
    const settings = parseChatSettings(search);
    const messages = shallowRef<ChatMessageView[]>([]);
    const controller = new ChatController({ onMessages: (next) => { messages.value = next; } }, settings);
    return { controller, render: (status, debug) => h(ChatWidget, { messages: messages.value, settings, status, debug }) };
  },
});

export const widgetTemplates = { follow: followTemplate, gift: giftTemplate, chat: chatTemplate, share: shareTemplate, subscribe: subscribeTemplate };
export type WidgetKind = keyof typeof widgetTemplates;

