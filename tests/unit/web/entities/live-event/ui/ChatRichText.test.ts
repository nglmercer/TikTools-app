import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { clearEmoteImageFailures } from '../../../../../../src/web/entities/live-event/lib/emote-failures.ts';
import ChatRichText from '../../../../../../src/web/entities/live-event/ui/ChatRichText.vue';

enableAutoUnmount(afterEach);
beforeEach(clearEmoteImageFailures);
describe('ChatRichText image fallback', () => {
  it('replaces failed images immediately and allows a new URL', async () => {
    const wrapper = mountWithPlugins(ChatRichText, { props: { text: '[custom]', emotes: [{ id: '123', shortcode: '[custom]', imageUrl: 'https://cdn.example/old.png' }] } });
    await wrapper.get('img').trigger('error');
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.text()).toBe('[custom]');
    await wrapper.setProps({ emotes: [{ id: '123', shortcode: '[custom]', imageUrl: 'https://cdn.example/new.png' }] });
    expect(wrapper.get('img').attributes('src')).toContain('new.png');
    const knownBroken = mountWithPlugins(ChatRichText, { props: { text: '', emotes: [{ id: '123', imageUrl: 'https://cdn.example/old.png' }] } });
    expect(knownBroken.find('img').exists()).toBe(false);
    expect(knownBroken.get('[role="img"]').attributes('aria-label')).toBe('TikTok emote');
  });
  it('preserves mixed text while rendering metadata without fabricated shortcodes', () => {
    const wrapper = mountWithPlugins(ChatRichText, { props: { text: 'Hi 👨‍👩‍👦 世界', emotes: [{ id: '123', imageUrl: 'https://cdn.example/a.png' }] } });
    expect(wrapper.text()).toBe('Hi 👨‍👩‍👦 世界');
    expect(wrapper.get('img').attributes('alt')).toBe('TikTok emote');
  });
  it('blocks plaintext artwork with an accessible neutral fallback', () => {
    const wrapper = mountWithPlugins(ChatRichText, { props: { text: '', emotes: [{ id: '123', imageUrl: 'http://cdn.example/a.png' }] } });
    expect(wrapper.find('img').exists()).toBe(false);
    expect(wrapper.get('[role="img"]').attributes('aria-label')).toBe('TikTok emote');
  });
});
