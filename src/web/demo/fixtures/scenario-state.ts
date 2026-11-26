import type { BehaviorSnapshot } from '../../../automation/behavior/types.ts';
// The demo host serves the same host-owned catalog the real host stamps
// into every behavior snapshot, so the template gallery is not empty.
import hostTemplates from '../../../../crates/tiktools-core/src/services/builtin-templates.json';
import { baseHostState } from './base-host.ts';
import type { ScenarioId } from '../scenarios.ts';
import { demoViewers, demoGifts } from './audience.ts';

export const DEMO_AT = 1_760_000_000_000;
const text = (value: string) => ({ default: value, i18key: `demo.${value.replaceAll(/[^a-zA-Z]/g, '').slice(0, 40)}` });
export const logType = {
  id: 'core.log', version: 1,
  title: { default: 'Write to the log', i18key: 'automation.action.core.log.title' },
  description: { default: 'Leave a line in the automation log.', i18key: 'automation.action.core.log.description' },
  tag: 'log', source: { kind: 'builtin' }, requiredCapabilities: [],
  fields: [{ key: 'message', kind: 'textarea', label: { default: 'Message', i18key: 'automation.action.core.log.field.message.label' }, value: 'Hello {{ user.nickname }}', template: true }],
} satisfies BehaviorSnapshot['actionTypes'][number];

const fetchType = {
  id: 'core.fetch', version: 1,
  title: { default: 'Call a URL', i18key: 'automation.action.core.fetch.title' },
  description: { default: 'POST or GET event data to a server or webhook.', i18key: 'automation.action.core.fetch.description' },
  tag: 'fetch', source: { kind: 'builtin' }, requiredCapabilities: ['http.request'],
  fields: [
    { key: 'method', kind: 'select', label: { default: 'Method', i18key: 'automation.action.core.fetch.field.method.label' }, value: 'POST', options: ['GET', 'POST', 'PUT', 'DELETE'].map((value) => ({ value, label: { default: value, i18key: `automation.action.core.fetch.field.method.option.${value}` } })) },
    { key: 'url', kind: 'text', label: { default: 'URL', i18key: 'automation.action.core.fetch.field.url.label' }, value: 'https://', placeholder: 'https://hooks.example.com/live', template: true },
    { key: 'headers', kind: 'keyvalue', label: { default: 'Headers', i18key: 'automation.action.core.fetch.field.headers.label' }, value: 'content-type=application/json', template: true, advanced: true },
    { key: 'body', kind: 'textarea', label: { default: 'Body', i18key: 'automation.action.core.fetch.field.body.label' }, value: '{\n  "viewer": "{{ event.user.uniqueId }}",\n  "type": "{{ event.type }}"\n}', template: true, showIf: { key: 'method', notEquals: ['GET'] } },
  ],
} satisfies BehaviorSnapshot['actionTypes'][number];

const audioType = {
  id: 'audio.play', version: 1,
  title: { default: 'Play a sound', i18key: 'automation.action.audio.play.title' },
  description: { default: 'Play a local audio file without copying it into TikTools.', i18key: 'automation.action.audio.play.description' },
  tag: 'audio', source: { kind: 'builtin' }, requiredCapabilities: ['audio.play'],
  fields: [
    { key: 'file', kind: 'media', label: { default: 'Audio file', i18key: 'automation.action.audio.play.field.file.label' }, value: '' },
    { key: 'volume', kind: 'range', label: { default: 'Volume', i18key: 'automation.action.audio.play.field.volume.label' }, value: '1', min: 0, max: 1, step: 0.05 },
  ],
} satisfies BehaviorSnapshot['actionTypes'][number];

const demoSpeakType = {
  id: 'demo.audio.speak', version: 1,
  title: { default: 'Speak with Demo Audio', i18key: 'demo.audio.speak.title' },
  description: { default: 'Simulated text-to-speech action owned by the demo audio plugin.', i18key: 'demo.audio.speak.description' },
  tag: 'tts', source: { kind: 'plugin', pluginId: 'demo.audio' }, requiredCapabilities: ['http.request'],
  fields: [
    { key: 'text', kind: 'text', label: { default: 'Text', i18key: 'demo.audio.speak.text.label' }, value: '{{ event.data.comment }}', template: true },
    { key: 'voice', kind: 'select', label: { default: 'Voice', i18key: 'demo.audio.speak.voice.label' }, value: 'narrator', options: [{ value: 'narrator', label: { default: 'Narrator', i18key: 'demo.audio.speak.voice.narrator' } }] },
    { key: 'language', kind: 'text', label: { default: 'Language', i18key: 'demo.audio.speak.language.label' }, value: 'en' },
  ],
} satisfies BehaviorSnapshot['actionTypes'][number];

const demoActionTranslations = {
  en: {
    'demo.audio.speak.title': 'Speak with Demo Audio',
    'demo.audio.speak.description': 'Simulated text-to-speech action owned by the demo audio plugin.',
    'demo.audio.speak.text.label': 'Text',
    'demo.audio.speak.voice.label': 'Voice',
    'demo.audio.speak.voice.narrator': 'Narrator',
    'demo.audio.speak.language.label': 'Language',
  },
  es: {
    'demo.audio.speak.title': 'Hablar con Demo Audio',
    'demo.audio.speak.description': 'Acción TTS simulada del plugin de audio del demo.',
    'demo.audio.speak.text.label': 'Texto',
    'demo.audio.speak.voice.label': 'Voz',
    'demo.audio.speak.voice.narrator': 'Narrador',
    'demo.audio.speak.language.label': 'Idioma',
  },
};
export function scenarioState(id: ScenarioId) {
  // Only the feed screen actually ingests live events. Seeding every scenario
  // as connected made the global status bar claim a LIVE session on screens
  // that never established one (analytics, points, overlays, ...). Other
  // scenarios start offline and connect through the normal live.connect path
  // on first playback; the creator record stays seeded for their selectors.
  const live = id === 'feed';
  const snapshot: BehaviorSnapshot = {
    actions: [
      { schemaVersion: 2, id: 'welcome-action', name: 'Welcome', typeId: 'core.log', enabled: true, config: { message: 'Hello {{ user.nickname }}' } },
      { schemaVersion: 2, id: 'webhook-action', name: 'Notify webhook', typeId: 'core.fetch', enabled: true, config: { method: 'POST', url: 'https://hooks.example.test/live', headers: 'content-type=application/json', body: '{"viewer":"{{ event.user.uniqueId }}"}' } },
      { schemaVersion: 2, id: 'sound-action', name: 'Play applause', typeId: 'audio.play', enabled: true, config: { file: '/demo/applause.ogg', volume: '1', overlap: 'allow' } },
      { schemaVersion: 2, id: 'speak-action', name: 'Speak chat', typeId: 'demo.audio.speak', enabled: true, config: { text: '{{ event.data.comment }}', voice: 'narrator', language: 'en' } },
    ],
    events: [{ schemaVersion: 1, id: 'welcome-event', name: 'Welcome chat', enabled: true, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'global', actionIds: ['welcome-action'], runMode: 'all' }],
    actionTypes: [logType, fetchType, audioType, demoSpeakType], plugins: [{ installed: true, enabled: true, available: true, running: true,
      descriptor: { id: 'demo.audio', name: text('Demo Audio'), version: '1.0.0', description: text('Simulated audio integration; no service or audio device is contacted.'), dependency: text('Demo runtime'), permissions: ['audio.play'], actionTypeIds: ['demo.audio.speak'], eventTypeIds: [], hasSettings: true, hasConnectionProbe: true } }],
    eventTypes: [], pluginTemplates: [], pluginPages: [], translations: demoActionTranslations, hostTemplates,
  };
  const state = baseHostState({
    automationSnapshot: snapshot as unknown as Record<string, unknown>,
    liveStatus: { connected: live, uniqueId: live ? 'demo_creator' : null, roomId: 'demo-room', connectionId: 'demo-connection', native: false },
    creator: { uniqueId: 'demo_creator', roomId: 'demo-room', nickname: 'Demo Creator', avatarUrl: null, title: 'Fictional LIVE', lastConnected: DEMO_AT, connectCount: 1 },
    recentCreators: [{ uniqueId: 'demo_creator', nickname: 'Demo Creator', avatarUrl: null, roomId: 'demo-room', lastConnected: DEMO_AT, connectCount: 1 }],
    leaderboard: demoViewers(DEMO_AT),
    gifts: demoGifts.map((gift) => ({ ...gift })),
    typedGlobals: { greeting: { type: 'text', value: 'Hello' }, goal: { type: 'number', value: 100 } }, globals: { greeting: 'Hello', goal: '100' },
    sync: { signedIn: true, enabled: true, entries: [], resolved: [] },
    variableComparisons: [{ name: 'greeting', review: 'demo-review-1', local: { type: 'text', value: 'Hello' }, cloud: { type: 'text', value: 'Welcome' }, cloudDeleted: false, localSecret: false, blockedLocal: false, pendingStatus: null, canSend: true, canApply: true, canWithdraw: false }],
    pluginSettings: { 'demo.audio': { schema: { type: 'object', properties: { volume: { type: 'number', minimum: 0, maximum: 100, default: 50 }, enabled: { type: 'boolean', default: true } } }, values: { volume: 50, enabled: true } } },
    pluginHealth: { 'demo.audio': { ok: true, latencyMs: 2 } },
  });
  return { state, snapshot };
}
