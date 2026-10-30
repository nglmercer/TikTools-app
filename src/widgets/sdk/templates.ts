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
import LikesWidget from '../likes/LikesWidget.vue';
import RankingWidget from '../ranking/RankingWidget.vue';
import RevealWidget from '../ranking/RevealWidget.vue';
import { RevealController, type RevealResult } from '../shared/reveal-controller.ts';
import { RankingController, type RankingSnapshot } from '../shared/ranking-controller.ts';
import { GiftController, type GiftAlertView } from '../shared/gift-controller.ts';
import { ChatController, type ChatMessageView } from '../shared/chat-controller.ts';
import { type LikeBoardEntry } from '../shared/likes-controller.ts';
import { NativeLikesController } from '../shared/native-likes-controller.ts';
import { parseFollowSettings, parseShareSettings, parseSubscribeSettings, parseGiftSettings, parseChatSettings, parseLikesSettings } from '../shared/config.ts';
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
  const editorSections: WidgetEditorSection[] = [
    { id: 'layers', groups: [{ controls: ['layers'] }] },
    { id: 'card', groups: [
      { title: 'appearance', controls: ['background', 'accent', 'radius', 'borderColor', 'borderWidth', 'shadow', 'opacity'] },
      { title: 'layout', controls: ['align', 'autoWidth', 'width', 'padding', 'gap'] },
    ] },
  ];
  return {
    defaultLayers: [
      ...(kind === 'gift' ? [{ id: 'art', kind: 'art' as const }] : []),
      { id: 'avatar', kind: 'avatar' as const },
      ...fields.map((field) => ({ id: `field:${field}`, kind: 'text' as const, field })),
    ],
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
const giftPreviewAvatar = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="32" fill="#534735"/><circle cx="32" cy="25" r="11" fill="#ffe7a0"/><path d="M10 62c2-16 10-24 22-24s20 8 22 24" fill="#ffe7a0"/></svg>',
)}`;
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
  samples: () => makeTestGiftCombo(5, { user: { avatarUrl: giftPreviewAvatar } }),
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

const likesSchema = templateSchema('likes', alertDesign('#fb7185', { padding: 16, width: 360 }), ['name', 'username', 'count']);

export const likesTemplate = defineWidget({
  id: 'likes',
  schema: likesSchema,
  topics: ['live.likes-ranking-full', 'live.disconnected', 'event.gap'],
  resetOnDisconnect: true,
  samples: () => [{ topic: 'live.likes-ranking-full', data: { creatorId: 'preview', roomId: 'preview', leaders: [
    { userId: 'u2', uniqueId: 'ben', nickname: 'Ben', rank: 1, score: 34 },
    { userId: 'u1', uniqueId: 'ana', nickname: 'Ana', rank: 2, score: 12 },
    { userId: 'u3', uniqueId: 'cid', nickname: 'Cid', rank: 3, score: 7 },
  ] } },
  ],
  create(search) {
    const settings = parseLikesSettings(search);
    const entries = shallowRef<LikeBoardEntry[]>([]);
    const handlers = { onBoard: (next: LikeBoardEntry[]) => { entries.value = [...next]; } };
    const degraded = shallowRef(false);
    const controller = new NativeLikesController(handlers, settings.maxEntries, (value) => { degraded.value = value; });
    return { controller, render: (status, debug) => h(LikesWidget, { entries: entries.value, settings, status, debug, degraded: degraded.value }) };
  },
});

// Rankings render a title and rows, not alert layers: offer only the
// title text and card controls the ranking widget actually applies.
const rankingBase = templateSchema('ranking', alertDesign('#fb7185', { padding: 18, width: 420 }), ['name', 'count']);
const rankingSchema: WidgetTemplateSchema = {
  ...rankingBase,
  defaultLayers: [{ id: 'field:title', kind: 'text', field: 'title' }],
  textFields: ['title'],
  requiredTextFields: [],
  editorSections: [
    { id: 'text', groups: [{ title: 'fields', controls: ['textFields'] }, { title: 'textAppearance', controls: ['textColor'] }] },
    ...rankingBase.editorSections.filter((section) => section.id === 'card'),
  ],
};

export const rankingTemplate = defineWidget({
  id: 'ranking',
  schema: rankingSchema,
  topics: ['ranking.updated', 'competition.result', 'event.gap'],
  resetOnDisconnect: true,
  samples: () => [{ topic: 'ranking.updated', data: { snapshot: { rankingId: 'preview', schemaVersion: 1, creatorId: 'preview', roomId: 'preview', revision: 1, integrityStatus: 'unknown', paused: false, entries: [{ key: 'Ana', rank: 1, score: 125 }, { key: 'Ben', rank: 2, score: 80 }, { key: 'Cid', rank: 3, score: 45 }], totalScore: 250, totalParticipants: 3 } } }],
  create(search) {
    const params = new URLSearchParams(search);
    const rankingId = params.get('rankingId') || 'preview';
    const requested = params.get('visualization');
    if (requested === 'roulette') {
      const result = shallowRef<RevealResult | null>(null);
      const controller = new RevealController(rankingId, (next) => { result.value = next; }, params.get('competitionId') || undefined);
      return { controller, render: (status, debug) => h(RevealWidget, { result: result.value, status, debug }) };
    }
    const visualization = requested === 'bar' || requested === 'pie' || requested === 'donut' || requested === 'poll' ? requested : 'ranking';
    const showLabels = params.get('showLabels') !== 'false';
    const showPercentages = params.get('showPercentages') === 'true';
    const showLegend = params.get('showLegend') !== 'false';
    const snapshot = shallowRef<RankingSnapshot | null>(null);
    const controller = new RankingController(rankingId, (next) => { snapshot.value = next; });
    return { controller, render: (status, debug) => h(RankingWidget, { snapshot: snapshot.value, visualization, status, debug, showLabels, showPercentages, showLegend }) };
  },
});

export const widgetTemplates = { follow: followTemplate, gift: giftTemplate, chat: chatTemplate, share: shareTemplate, subscribe: subscribeTemplate, likes: likesTemplate, ranking: rankingTemplate };
export type WidgetKind = keyof typeof widgetTemplates;
/** Standalone registry; ranking visualizations share the editor design schema. */
export const standaloneWidgetTemplates = { ...widgetTemplates };
