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
