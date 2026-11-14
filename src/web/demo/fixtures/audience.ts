import type { GiftCatalogEntry, UiEvent, ViewerRecord } from '../../../shared/messages.ts';
import type { AutomationEvent, JsonObject } from '../../../automation/types.ts';
import type { DemoLocale } from '../scenarios.ts';
import roseUrl from '../assets/rose.png';

// All identities and activity are fictional and repeat deterministically.
const audience = [
  { uniqueId: 'luna', nickname: 'Luna', points: 120, totalChats: 40, totalCoins: 10, totalLikes: 5, totalShares: 1, isSubscriber: true },
  { uniqueId: 'milo', nickname: 'Milo', points: 85, totalChats: 18, totalCoins: 5, totalLikes: 30, totalShares: 2, isSubscriber: false },
  { uniqueId: 'sofia', nickname: 'Sofía', points: 64, totalChats: 12, totalCoins: 8, totalLikes: 15, totalShares: 1, isSubscriber: false },
  { uniqueId: 'kai', nickname: 'Kai', points: 42, totalChats: 9, totalCoins: 3, totalLikes: 20, totalShares: 1, isSubscriber: true },
  { uniqueId: 'nora', nickname: 'Nora', points: 28, totalChats: 6, totalCoins: 2, totalLikes: 10, totalShares: 0, isSubscriber: false },
  { uniqueId: 'leo', nickname: 'Leo', points: 15, totalChats: 3, totalCoins: 1, totalLikes: 8, totalShares: 0, isSubscriber: false },
] as const;
type ViewerId = typeof audience[number]['uniqueId'];
export function demoViewers(at: number): ViewerRecord[] {
  return audience.map((viewer, index) => ({ ...viewer, userId: `demo-viewer-${index + 1}`, level: Math.floor(viewer.points / 100) + 1, firstSeen: at - (index + 1) * 86400000, lastSeen: at }));
}
export const demoGifts: readonly GiftCatalogEntry[] = [{ id: 'demo-rose', name: 'Rose', diamondCount: 1, iconUrl: roseUrl }];

type Activity = { viewer: ViewerId } & (
  { type: 'chat'; message: number } | { type: 'gift' | 'like'; count: number } | { type: 'join' | 'follow' | 'share' }
);
const activity: readonly Activity[] = [
  { viewer: 'luna', type: 'chat', message: 0 },
  { viewer: 'milo', type: 'like', count: 12 },
  { viewer: 'sofia', type: 'gift', count: 1 },
  { viewer: 'kai', type: 'chat', message: 1 },
  { viewer: 'nora', type: 'join' },
  { viewer: 'leo', type: 'follow' },
  { viewer: 'milo', type: 'chat', message: 2 },
  { viewer: 'luna', type: 'like', count: 5 },
  { viewer: 'nora', type: 'gift', count: 3 },
  { viewer: 'sofia', type: 'chat', message: 3 },
  { viewer: 'kai', type: 'share' },
  { viewer: 'leo', type: 'chat', message: 4 },
  { viewer: 'nora', type: 'like', count: 20 },
  { viewer: 'milo', type: 'gift', count: 2 },
  { viewer: 'luna', type: 'chat', message: 1 },
  { viewer: 'sofia', type: 'follow' },
  { viewer: 'kai', type: 'gift', count: 5 },
  { viewer: 'leo', type: 'like', count: 8 },
  { viewer: 'nora', type: 'chat', message: 5 },
  { viewer: 'milo', type: 'share' },
  { viewer: 'sofia', type: 'like', count: 15 },
  { viewer: 'kai', type: 'join' },
  { viewer: 'leo', type: 'gift', count: 1 },
  { viewer: 'luna', type: 'chat', message: 6 },
];
export const DEMO_ACTIVITY_LENGTH = activity.length;
const comments: Record<DemoLocale, readonly string[]> = {
  en: ['Hello TikTools!', '!hello', 'Hello from Lima 👋', 'How do I set up audience points?', 'The overlay looks great ✨', 'Let’s reach the goal!', 'Thanks for joining us 🌹'],
  es: ['Hello TikTools!', '!hello', 'Saludos desde Lima 👋', '¿Cómo configuro los puntos?', 'El overlay se ve genial ✨', '¡Vamos por la meta!', 'Gracias por acompañarnos 🌹'],
  pt: ['Hello TikTools!', '!hello', 'Olá de Lima 👋', 'Como configuro os pontos?', 'O overlay ficou ótimo ✨', 'Vamos alcançar a meta!', 'Obrigado por estar aqui 🌹'],
  fr: ['Hello TikTools!', '!hello', 'Bonjour de Lima 👋', 'Comment configurer les points ?', 'La superposition est superbe ✨', 'Atteignons l’objectif !', 'Merci d’être là 🌹'],
  zh: ['Hello TikTools!', '!hello', '来自利马的问候 👋', '如何设置观众积分？', '叠加层看起来很棒 ✨', '一起达成目标！', '感谢大家的陪伴 🌹'],
  ja: ['Hello TikTools!', '!hello', 'リマからこんにちは 👋', 'ポイントはどう設定しますか？', 'オーバーレイが素敵です ✨', '目標を達成しましょう！', 'ご参加ありがとうございます 🌹'],
};

export function demoActivity(tick: number, locale: DemoLocale) {
  const entry = activity[(tick - 1) % activity.length]!;
  const viewer = audience.find((user) => user.uniqueId === entry.viewer)!;
  const msgId = `demo-message-${tick}`;
  const event: UiEvent = { kind: 'chat', author: viewer.uniqueId, nickname: viewer.nickname, isSubscriber: viewer.isSubscriber, text: '', msgId, correlationId: msgId, feedCursor: tick, roomId: 'demo-room', creatorId: 'demo_creator', sourceType: `tiktok.${entry.type}` };
  const common = { msgId, isHistory: false, method: `demo.${entry.type}` };
  let data: JsonObject;
  if (entry.type === 'chat') {
    event.text = comments[locale][entry.message]!;
    data = { ...common, comment: event.text };
  } else if (entry.type === 'gift') {
    event.kind = 'gift'; event.text = 'Rose'; event.i18nKey = 'giftSent';
    event.giftDetails = { giftId: 'demo-rose', name: 'Rose', count: entry.count, diamonds: entry.count, imageUrl: roseUrl, groupId: `demo-gift-${tick}`, repeatEnd: true, streakable: true };
    data = { ...common, giftId: 'demo-rose', giftName: 'Rose', giftIconUrl: roseUrl, diamondCount: 1, repeatCount: entry.count, comboCount: entry.count, groupId: `demo-gift-${tick}`, repeatEnd: true, streakable: true, aggregation: { cumulative: entry.count, increment: entry.count, diamonds: entry.count } };
  } else if (entry.type === 'like') {
    event.kind = 'like'; event.text = `${entry.count} likes`; event.i18nKey = 'likeSent'; event.likeCount = entry.count;
    data = { ...common, count: entry.count };
  } else if (entry.type === 'join') {
    event.kind = 'member'; event.text = 'Joined the LIVE'; event.i18nKey = 'joinedLive';
    data = { ...common, action: 1, memberCount: audience.length };
  } else {
    event.kind = 'social'; event.text = entry.type === 'follow' ? 'Followed the creator' : 'Shared the LIVE'; event.i18nKey = entry.type === 'follow' ? 'followedCreator' : 'sharedLive';
    data = { ...common, action: entry.type === 'follow' ? 1 : 3, followCount: entry.type === 'follow' ? 1 : 0, shareCount: entry.type === 'share' ? 1 : 0 };
  }
  return { event, data, type: `tiktok.${entry.type}` as AutomationEvent['type'] };
}
