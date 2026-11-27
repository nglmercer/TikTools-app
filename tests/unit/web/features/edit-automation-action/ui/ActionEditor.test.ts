import { testMessage } from '@/shared/i18n/test-message.ts';
import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { LiveAction } from '../../../../../../src/automation/behavior/types.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { actionType, text } from '../../../../support/automation.ts';
import ActionEditor from '../../../../../../src/web/features/edit-automation-action/ui/ActionEditor.vue';

enableAutoUnmount(afterEach);
const t = (key: string): string => testMessage('en', key);

beforeEach(() => {
  localStorage.clear();
});

const optionType = actionType({
  id: 'test.hazard',
  title: text('Spawn hazard'),
  description: text('Spawns a hazard.'),
  tag: 'hazard',
  configSchema: { type: 'object', properties: {} },
  uiHints: { fields: { hazard: { optionsFrom: 'host.gifts' } } },
});

const action: LiveAction = {
  schemaVersion: 2,
  id: 'a1',
  name: 'Invoke tumbleweed',
  enabled: true,
  typeId: 'test.hazard',
  config: {},
};

function mountEditor(optionErrors: Record<string, string>) {
  return mountWithPlugins(ActionEditor, {
    attachTo: document.body,
    props: {
      action,
      actionTypes: [optionType],
      testRuns: [],
      globals: {},
      actionOptions: {},
      actionOptionErrors: optionErrors,
    },
  });
}

const emptyTextType = actionType({
  id: 'plugin.search-play',
  title: text('Search and play'),
  description: text('Searches and plays a match.'),
  tag: 'music',
  source: { kind: 'plugin', pluginId: 'plugin' },
  fields: [
    { key: 'text', label: text('Search text'), kind: 'text', value: '', template: true, placeholder: 'Daft Punk' },
  ],
});

it('renders an empty default for a templated text field as empty, not "true"', async () => {
  const wrapper = mountWithPlugins(ActionEditor, {
    attachTo: document.body,
    props: {
      action: { schemaVersion: 2, id: 'a2', name: 'Search and play', enabled: true, typeId: 'plugin.search-play', config: { text: '', kind: 'song' } },
      actionTypes: [emptyTextType],
      testRuns: [],
      globals: {},
      actionOptions: {},
      actionOptionErrors: {},
    },
  });
  await flushPromises();
  const input = wrapper.findAll<HTMLInputElement>('input').find((el) => el.element.placeholder === 'Daft Punk');
  expect(input!.element.value).toBe('');
});

it('keeps the enable switch in the General section header and mutes an inactive action', async () => {
  const wrapper = mountEditor({});
  const input = wrapper.get<HTMLInputElement>('input[role="switch"]');
  expect(input.element.closest('.base-section__actions')).not.toBeNull();
  expect(input.element.closest('.base-section__header')?.textContent).toContain(t('automations.event.general'));
  expect(wrapper.get('.action-editor__columns').classes()).not.toContain('action-editor__columns--inactive');
  await input.setValue(false);
  expect(wrapper.text()).toContain(t('automations.editor.disabled'));
  expect(wrapper.get('.action-editor__columns').classes()).toContain('action-editor__columns--inactive');
});

describe('ActionEditor option errors', () => {
  it('shows one compact row without an inline details section', async () => {
    const wrapper = mountEditor({ 'host.gifts': 'HTTP request failed' });
    await flushPromises();
    expect(wrapper.find('.base-alert__title').text()).toBe(t('automations.action.optionsFailed').trim());
    expect(wrapper.find('.base-alert').classes()).toContain('base-alert--compact');
    expect(wrapper.find('.base-disclosure').exists()).toBe(false);
  });

  it('reveals the raw messages on hover', async () => {
    const wrapper = mountEditor({ 'host.gifts': 'HTTP request failed' });
    await flushPromises();
    vi.useFakeTimers();
    try {
      const alert = wrapper.get('.base-alert');
      alert.element.dispatchEvent(new Event('focusin', { bubbles: true }));
      vi.advanceTimersByTime(600);
      expect(document.getElementById('app-tooltip')?.textContent).toBe('HTTP request failed');
    } finally {
      vi.useRealTimers();
    }
  });
});
