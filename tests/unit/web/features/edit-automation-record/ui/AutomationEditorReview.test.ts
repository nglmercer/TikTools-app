import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { AutomationEdit, AutomationReview } from '../../../../../../src/web/features/edit-automation-record/model/automation-editor.ts';
import AutomationEditorReview from '../../../../../../src/web/features/edit-automation-record/ui/AutomationEditorReview.vue';
import { testMessage } from '@/shared/i18n/test-message.ts';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

const base = { schemaVersion: 2, id: 'same', name: 'Starting event', enabled: true, trigger: 'tiktok.gift' };
const candidate = { ...base, name: 'My retained draft' };
const current = { ...base, name: 'Host changed name' };
const edit = { revision: '1', base, candidate } as unknown as AutomationEdit;
const review = { current: { record: current, revision: '2' } } as unknown as AutomationReview;

function mountReview(props: Record<string, unknown> = {}) {
  return mountWithPlugins(AutomationEditorReview, {
    attachTo: document.body,
    props: { open: true, edit, review, busy: false, error: '', canAdopt: true, ...props },
  });
}

async function expandCard(wrapper: ReturnType<typeof mountReview>, index: number) {
  const card = wrapper.findAll('.change-review__card')[index]!;
  await card.get('.change-review__chevron').trigger('click');
  await nextTick();
  return card;
}

async function codeMode(wrapper: ReturnType<typeof mountReview>) {
  const tab = wrapper.findAll('[role="radio"]').find((radio) => radio.text() === t('changeReview.code'))!;
  await tab.trigger('click');
  await nextTick();
}

describe('AutomationEditorReview', () => {
  it('shows the draft and host diffs as two change cards', async () => {
    const wrapper = mountReview();
    await nextTick();
    const cards = wrapper.findAll('.change-review__card');
    expect(cards).toHaveLength(2);
    expect(wrapper.text()).toContain(t('jsonView.draftChanges'));
    expect(wrapper.text()).toContain(t('jsonView.hostChanges'));
  });

  it('shows the semantic field diff inside the expanded card', async () => {
    const wrapper = mountReview();
    await nextTick();
    const card = await expandCard(wrapper, 0);
    expect(card.text()).toContain('name');
    expect(card.text()).toContain('Starting event');
    expect(card.text()).toContain('My retained draft');
  });

  it('shows the pretty JSON of both sides in code mode without opening another dialog', async () => {
    const wrapper = mountReview();
    await nextTick();
    const card = await expandCard(wrapper, 0);
    await codeMode(wrapper);
    expect(card.find('.base-json-diff').exists()).toBe(true);
    expect(card.text()).toContain('My retained draft');
    expect(wrapper.find('.review-source-dialog').exists()).toBe(false);
  });

  it('marks the host diff as deleted when the host record is gone', async () => {
    const wrapper = mountReview({ review: { current: { record: null, revision: '2' } } });
    await nextTick();
    const cards = wrapper.findAll('.change-review__card');
    expect(cards[1]!.classes()).toContain('change-review__card--deleted');
    expect(wrapper.text()).toContain(t('automations.conflict.recreateLead'));
  });
});
