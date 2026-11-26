import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { testMessage } from '@/shared/i18n/test-message.ts';
import ChangeReviewWorkspace from '../../../../../../src/web/features/review-shared/ui/ChangeReviewWorkspace.vue';
import type { ChangeReviewItem } from '../../../../../../src/web/features/review-shared/model/change-review.ts';

enableAutoUnmount(afterEach);
beforeEach(() => activateLocale('en'));
const t = (key: string): string => testMessage('en', key);

const before = { schemaVersion: 2, id: 'act-1', name: 'Invocar Pato', enabled: false, typeId: 'repo.game.spawn-enemy', config: { quantity: 1 } };
const after = { ...before, enabled: true, config: { quantity: 2 } };
const items: ChangeReviewItem[] = [
  { id: 'modified', title: 'Invocar Pato', subtitle: 'repo.game.spawn-enemy', before, after },
  { id: 'added', title: 'Minecraft gifts', before: undefined, after: { id: 'evt-1', name: 'Minecraft gifts' } },
  { id: 'deleted', title: 'R.E.P.O. gifts', before: { id: 'evt-2', name: 'R.E.P.O. gifts' }, after: undefined, leaving: true },
];

function mountWorkspace(props: Record<string, unknown> = {}) {
  return mountWithPlugins(ChangeReviewWorkspace, { attachTo: document.body, props: { items, ...props } });
}

describe('ChangeReviewWorkspace', () => {
  it('renders one collapsed card per item ordered added, modified, deleted', async () => {
    const wrapper = mountWorkspace();
    await nextTick();
    const cards = wrapper.findAll('.change-review__card');
    expect(cards).toHaveLength(3);
    expect(cards[0]!.classes()).toContain('change-review__card--added');
    expect(cards[1]!.classes()).toContain('change-review__card--modified');
    expect(cards[2]!.classes()).toContain('change-review__card--deleted');
    expect(wrapper.find('.change-review__fields').exists()).toBe(false);
  });

  it('expands a card inline with the chevron and shows the semantic field diff', async () => {
    const wrapper = mountWorkspace();
    await nextTick();
    const card = wrapper.findAll('.change-review__card')[1]!;
    await card.get('.change-review__chevron').trigger('click');
    await nextTick();
    const text = card.text();
    expect(text).toContain('enabled');
    expect(text).toContain('config.quantity');
    expect(card.find('.base-json-diff').exists()).toBe(false);
  });

  it('switches the expanded card to the JSON diff in code mode', async () => {
    const wrapper = mountWorkspace();
    await nextTick();
    const card = wrapper.findAll('.change-review__card')[1]!;
    await card.get('.change-review__chevron').trigger('click');
    const codeTab = wrapper.findAll('[role="radio"]').find((radio) => radio.text() === t('changeReview.code'))!;
    await codeTab.trigger('click');
    await nextTick();
    expect(card.find('.base-json-diff').exists()).toBe(true);
    expect(card.find('.change-review__fields').exists()).toBe(false);
  });

  it('hides leaving items with the profile filter and reports them in the badge', async () => {
    const wrapper = mountWorkspace({ profileFilter: true });
    await nextTick();
    expect(wrapper.findAll('.change-review__card')).toHaveLength(2);
    expect(wrapper.get('.change-review__hidden-badge').text()).toContain('1');
    await wrapper.get(`[aria-label="${t('changeReview.onlyNew')}"]`).trigger('click');
    await nextTick();
    expect(wrapper.findAll('.change-review__card')).toHaveLength(3);
    expect(wrapper.find('.change-review__hidden-badge').exists()).toBe(false);
  });

  it('hides unchanged items until the eye toggle shows them', async () => {
    const same = { id: 'same', title: 'Same', before: { a: 1 }, after: { a: 1 } };
    const wrapper = mountWorkspace({ items: [items[0], same] });
    await nextTick();
    expect(wrapper.findAll('.change-review__card')).toHaveLength(1);
    await wrapper.get(`[aria-label="${t('changeReview.showUnchanged')}"]`).trigger('click');
    await nextTick();
    expect(wrapper.findAll('.change-review__card')).toHaveLength(2);
  });

  it('shows the empty state when nothing is visible', async () => {
    const wrapper = mountWorkspace({ items: [] });
    await nextTick();
    expect(wrapper.text()).toContain(t('changeReview.empty'));
  });
});
