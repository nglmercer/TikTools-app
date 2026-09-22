import { h, shallowRef } from 'vue';
import { defineWidget, type WidgetEditorSection, type WidgetStyle, type WidgetTemplateSchema, type WidgetTemplateToken } from './template.ts';
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
import { coreTextFields, textDefaults, type TextField } from './text.ts';

function alertDesign(accent: string, options: {
  background?: string;
  textColor?: string;
  radius?: number;
  padding?: number;
  gap?: number;
  width?: number;
  avatarSize?: number;
} = {}): WidgetStyle {
  return {
    background: options.background ?? '#16161d',
    textColor: options.textColor ?? '#f5f5f7',
    accent,
    radius: options.radius ?? 16,
    borderColor: '#2a2a33',
    borderWidth: 1,
    opacity: 100,
    padding: options.padding ?? 16,
    gap: options.gap ?? 16,
    width: options.width ?? 440,
    avatar: { size: options.avatarSize ?? 56 },
    badge: { fontSize: 11, fontWeight: 700, letterSpacing: 1.5 },
  };
}

function templateSchema(
  kind: keyof typeof textDefaults,
  defaultDesign: WidgetStyle,
  tokens: readonly WidgetTemplateToken[],
): WidgetTemplateSchema {
  const fields = Object.keys(textDefaults[kind]) as TextField[];
  const headingFields = fields.filter((field) => field === 'title' || field === 'streakTitle');
  const editorSections: WidgetEditorSection[] = [
    { id: 'text', groups: [
      { title: 'fields', controls: ['textFields', 'addTextField'] },
      { title: 'textAppearance', controls: ['textColor'] },
    ] },
    { id: 'card', groups: [
      { title: 'appearance', controls: ['background', 'accent', 'radius', 'borderColor', 'borderWidth', 'shadow', 'opacity'] },
      { title: 'layout', controls: ['align', 'autoWidth', 'width', 'padding', 'gap'] },
    ] },
    { id: 'image', groups: [
      { controls: ['avatarVisible', 'avatarSize', 'avatarRadius', 'avatarBorderColor', 'avatarBorderWidth'] },
    ] },
    ...(headingFields.length > 0 ? [{ id: 'heading' as const, textFields: headingFields, groups: [
      { title: 'appearance' as const, controls: ['headingVisible' as const, 'headingColor' as const, 'headingFontSize' as const, 'headingWeight' as const, 'headingSpacing' as const] },
    ] }] : []),
  ];
  return {
    textFields: fields,
    requiredTextFields: coreTextFields[kind],
    tokens,
    editorSections,
    defaultDesign,
  };
}

const followSchema = templateSchema('follow', alertDesign('#22c55e'), ['name', 'username']);
const shareSchema = templateSchema('share', alertDesign('#22d3ee'), ['name', 'username']);
const subscribeSchema = templateSchema('subscribe', alertDesign('#fe2c55'), ['name', 'username']);
const giftSchema = templateSchema('gift', alertDesign('#f5c518', { padding: 18, width: 500, avatarSize: 34 }), ['name', 'gift', 'count', 'diamonds']);
const chatSchema = templateSchema('chat', alertDesign('#f5f5f7', {
  background: '#16161ddb',
  textColor: '#e8e8ec',
  radius: 12,
  padding: 12,
  gap: 8,
  width: 520,
  avatarSize: 28,
}), ['name', 'username', 'message']);

export const followTemplate = defineWidget({
  id: 'follow',
  schema: followSchema,
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
  schema: shareSchema,
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
  schema: subscribeSchema,
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
  schema: giftSchema,
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
  schema: chatSchema,
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
