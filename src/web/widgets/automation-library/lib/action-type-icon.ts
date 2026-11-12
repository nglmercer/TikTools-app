import { markRaw, type Component } from 'vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconStop from '@/shared/ui/icon/icons/IconStop.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import IconVolume from '@/shared/ui/icon/icons/IconVolume.vue';

// Small glyph per action family so the actions table scans faster than
// text alone. Keywords match type ids such as `core.log`,
// `peak.game.spawn-item` or `tts.speak`; anything unknown gets the
// generic plugin glyph.
const fallbackActionIcon = markRaw(IconPuzzle);
const actionTypeIcons: ReadonlyArray<{ match: string; icon: Component }> = [
  { match: 'hazard', icon: markRaw(IconStop) },
  { match: 'entity', icon: markRaw(IconUser) },
  { match: 'item', icon: markRaw(IconPlus) },
  { match: 'log', icon: markRaw(IconChat) },
  { match: 'point', icon: markRaw(IconCoins) },
  { match: 'coin', icon: markRaw(IconCoins) },
  { match: 'tts', icon: markRaw(IconVolume) },
  { match: 'speak', icon: markRaw(IconVolume) },
  { match: 'sound', icon: markRaw(IconVolume) },
];

export function actionTypeIcon(typeId: string): Component {
  const id = typeId.toLowerCase();
  return actionTypeIcons.find((entry) => id.includes(entry.match))?.icon ?? fallbackActionIcon;
}
