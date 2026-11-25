import type { Component } from 'vue';
import IconChart from './icons/IconChart.vue';
import IconChat from './icons/IconChat.vue';
import IconClock from './icons/IconClock.vue';
import IconCoins from './icons/IconCoins.vue';
import IconDiamond from './icons/IconDiamond.vue';
import IconGear from './icons/IconGear.vue';
import IconGlobe from './icons/IconGlobe.vue';
import IconInfo from './icons/IconInfo.vue';
import IconKey from './icons/IconKey.vue';
import IconKeyboard from './icons/IconKeyboard.vue';
import IconLink from './icons/IconLink.vue';
import IconLock from './icons/IconLock.vue';
import IconPuzzle from './icons/IconPuzzle.vue';
import IconRecord from './icons/IconRecord.vue';
import IconServer from './icons/IconServer.vue';
import IconShield from './icons/IconShield.vue';
import IconUser from './icons/IconUser.vue';
import IconVideo from './icons/IconVideo.vue';
import IconVolume from './icons/IconVolume.vue';
import IconWarning from './icons/IconWarning.vue';

/**
 * Host-owned plugin icon registry. Plugins may name one of these keys in a
 * manifest, but they never provide SVG or executable assets for the shell.
 * Unknown names fall back to conservative keyword heuristics and then to the
 * generic puzzle icon.
 */
const exactIcons: Readonly<Record<string, Component>> = {
  analytics: IconChart,
  audio: IconVolume,
  chat: IconChat,
  clock: IconClock,
  coins: IconCoins,
  connect: IconLink,
  connection: IconLink,
  diamond: IconDiamond,
  gear: IconGear,
  gift: IconDiamond,
  globe: IconGlobe,
  hotkey: IconKeyboard,
  info: IconInfo,
  key: IconKey,
  keyboard: IconKeyboard,
  link: IconLink,
  live: IconVideo,
  lock: IconLock,
  message: IconChat,
  network: IconGlobe,
  plugin: IconPuzzle,
  points: IconCoins,
  puzzle: IconPuzzle,
  radio: IconRecord,
  remote: IconServer,
  server: IconServer,
  settings: IconGear,
  shield: IconShield,
  stream: IconVideo,
  tts: IconVolume,
  user: IconUser,
  video: IconVideo,
  voice: IconVolume,
  volume: IconVolume,
  warning: IconWarning,
};

const heuristics: ReadonlyArray<readonly [string, Component]> = [
  ['voice', IconVolume],
  ['tts', IconVolume],
  ['audio', IconVolume],
  ['sound', IconVolume],
  ['video', IconVideo],
  ['stream', IconVideo],
  ['live', IconVideo],
  ['chat', IconChat],
  ['message', IconChat],
  ['network', IconGlobe],
  ['remote', IconServer],
  ['server', IconServer],
  ['radio', IconRecord],
  ['connect', IconLink],
  ['link', IconLink],
  ['hotkey', IconKeyboard],
  ['keyboard', IconKeyboard],
  ['gift', IconDiamond],
  ['point', IconCoins],
  ['coin', IconCoins],
  ['setting', IconGear],
  ['key', IconKey],
  ['shield', IconShield],
  ['user', IconUser],
  ['warn', IconWarning],
];

export function pluginIcon(name: string | undefined): Component {
  const key = name?.trim().toLowerCase() ?? '';
  if (!key) return IconPuzzle;
  const exact = exactIcons[key];
  if (exact) return exact;
  return heuristics.find(([needle]) => key.includes(needle))?.[1] ?? IconPuzzle;
}
