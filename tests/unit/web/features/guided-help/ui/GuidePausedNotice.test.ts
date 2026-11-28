import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import GuidePausedNotice from '../../../../../../src/web/features/guided-help/ui/GuidePausedNotice.vue';
import { pausedGuide } from '../../../../../../src/web/features/guided-help/model/paused-guide.ts';
import type { GuidePause, GuideSession } from '../../../../../../src/web/features/guided-help/model/types.ts';

enableAutoUnmount(afterEach);
beforeEach(async () => {
  await activateLocale('en');
  pausedGuide.value = undefined;
});
afterEach(() => {
  pausedGuide.value = undefined;
});

const session: GuideSession = {
  id: 'points',
  ready: Promise.resolve(true),
  done: Promise.resolve('dismissed'),
  active: true,
  stop: () => {},
  refresh: () => {},
  updateTaskState: () => {},
};

function pause(resume: () => GuideSession): GuidePause {
  return { id: 'points', stepId: 'leaderboard', title: 'Inspect committed points', cause: 'navigation', resume };
}

it('offers to resume a guide the app interrupted, and names the step it stopped on', async () => {
  const resume = vi.fn(() => session);
  pausedGuide.value = pause(resume);
  const wrapper = mountWithPlugins(GuidePausedNotice);
  expect(wrapper.text()).toContain('Guide paused at “Inspect committed points”');
  await wrapper.get('.base-notification__action').trigger('click');
  expect(resume).toHaveBeenCalledTimes(1);
  expect(pausedGuide.value).toBeUndefined();
});

it('shows nothing when no guide is paused, and dismissing forgets the pause', async () => {
  const wrapper = mountWithPlugins(GuidePausedNotice);
  expect(wrapper.text()).not.toContain('Guide paused');
  pausedGuide.value = pause(() => session);
  await wrapper.vm.$nextTick();
  await wrapper.get('.base-notification__dismiss').trigger('click');
  expect(pausedGuide.value).toBeUndefined();
});
