import { expect, it } from 'vitest';
import type { AutomationEdit } from '../../../../../../src/web/features/edit-automation-record/model/automation-editor.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import AutomationEditNotice from '../../../../../../src/web/features/edit-automation-record/ui/AutomationEditNotice.vue';

const t = (key: string) => i18n.global.t(key);
const edit = (autoResolved: boolean) => ({
  autoResolved,
  busy: false,
  reviewRequired: false,
  revision: '1',
  failure: null,
  message: '',
}) as AutomationEdit;

it('stays collapsed when there is nothing to report', () => {
  const wrapper = mountWithPlugins(AutomationEditNotice, {
    props: { edit: edit(false), hostChanged: false, memory: false },
  });
  expect(wrapper.find('.automation-edit-notice').exists()).toBe(false);
  expect(wrapper.text()).not.toContain(t('jsonView.reviewTable.docs'));
});

it('shows the waiting status as a floating notification', () => {
  const wrapper = mountWithPlugins(AutomationEditNotice, {
    props: { edit: { ...edit(false), busy: true }, hostChanged: false, memory: false },
  });
  expect(wrapper.text()).toContain(t('automations.conflict.waiting'));
  expect(wrapper.find('.base-notification').exists()).toBe(true);
});

it('shows an automatic rebase as a dismissible notification', async () => {
  const wrapper = mountWithPlugins(AutomationEditNotice, {
    attachTo: document.body,
    props: { edit: edit(true), hostChanged: false, memory: false },
  });
  const message = t('automations.conflict.autoResolved');
  expect(wrapper.text()).toContain(message);
  expect(wrapper.find('[role="status"]').exists()).toBe(true);
  await wrapper.get(`button[aria-label="${t('common.close')}"]`).trigger('click');
  expect(wrapper.text()).not.toContain(message);
});
