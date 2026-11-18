import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { AutomationEdit, AutomationReview } from '../model/automation-editor.ts';
import AutomationEditorReview from './AutomationEditorReview.vue';
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

async function openSource(wrapper: ReturnType<typeof mountReview>, index: number): Promise<string> {
  const buttons = wrapper.findAll('.review-table__action');
  await buttons[index]!.trigger('click');
  await nextTick();
  return wrapper.get('.review-source-dialog').text();
}

describe('AutomationEditorReview', () => {
  it('shows the three versions and the two diffs as five numbered rows', async () => {
    const wrapper = mountReview();
    await nextTick();
    expect(wrapper.findAll('tbody tr')).toHaveLength(5);
    expect(wrapper.findAll('.review-table__action')).toHaveLength(5);
    expect(wrapper.text()).toContain(t('automations.conflict.base'));
    expect(wrapper.text()).toContain(t('automations.conflict.candidate'));
    expect(wrapper.text()).toContain(t('automations.conflict.current'));
    expect(wrapper.find('a.review-docs-link').attributes('href')).toContain('developer/automations.html#expected-result-and-status');
  });

  it('keeps text for an absent base and a deleted host record in the numbered table', async () => {
    const wrapper = mountReview({ edit: { ...edit, base: null }, review: { current: { record: null, revision: '2' } } });
    await nextTick();
    expect(wrapper.findAll('tbody tr')).toHaveLength(5);
    expect(wrapper.text()).toContain(t('automations.conflict.absent'));
    expect(wrapper.text()).toContain(t('automations.conflict.deleted'));
  });

  it('shows the exact pretty JSON of each version inside the source dialog', async () => {
    const wrapper = mountReview();
    await nextTick();
    for (const [index, record] of [base, candidate, current].entries()) {
      const text = await openSource(wrapper, index);
      expect(text).toContain(JSON.stringify(record, null, 2));
    }
  });

  it('compares the starting version with the draft and with the host in diff rows', async () => {
    const wrapper = mountReview();
    await nextTick();
    const draftDiff = await openSource(wrapper, 3);
    const hostDiff = await openSource(wrapper, 4);
    expect(draftDiff).toContain('My retained draft');
    expect(hostDiff).toContain('Host changed name');
    expect(wrapper.findAll('.base-json-diff')).toHaveLength(1);
  });
});
