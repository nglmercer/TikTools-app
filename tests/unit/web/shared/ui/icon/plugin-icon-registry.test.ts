import { describe, expect, test } from 'vitest';

import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconServer from '@/shared/ui/icon/icons/IconServer.vue';
import IconVideo from '@/shared/ui/icon/icons/IconVideo.vue';
import IconVolume from '@/shared/ui/icon/icons/IconVolume.vue';
import { pluginIcon } from '@/shared/ui/icon/plugin-icon-registry.ts';

describe('pluginIcon', () => {
  test('maps exact host registry names to owned svg components', () => {
    expect(pluginIcon('voice')).toBe(IconVolume);
    expect(pluginIcon('video')).toBe(IconVideo);
    expect(pluginIcon('chat')).toBe(IconChat);
    expect(pluginIcon('remote-control')).toBe(IconServer);
  });

  test('uses conservative heuristics and falls back to puzzle', () => {
    expect(pluginIcon('chat-tts')).toBe(IconVolume);
    expect(pluginIcon('unknown-custom-icon')).toBe(IconPuzzle);
    expect(pluginIcon(undefined)).toBe(IconPuzzle);
  });
});
