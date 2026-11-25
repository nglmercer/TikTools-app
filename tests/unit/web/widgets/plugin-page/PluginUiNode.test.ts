import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, test } from 'vitest';

import type { PluginUiNode as PluginUiNodeContract } from '../../../../../src/plugin-ui/index.ts';
import type { PluginUiContext } from '@/entities/plugin/model/plugin-ui-context.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginUiNode from '@/widgets/plugin-page/ui/PluginUiNode.vue';

enableAutoUnmount(afterEach);

function context(): PluginUiContext {
  const drafts = new Map<string, Record<string, never>>();
  return {
    pluginId: 'tiktools.twitch',
    pluginName: 'Twitch Live',
    settings: {
      state: {
        schema: {
          type: 'object',
          properties: {
            channel: { type: 'string', title: { default: 'Twitch channel' } },
          },
        },
        uiHints: { fields: { channel: { placeholder: 'shroud' } } },
        values: {},
      },
      get() {},
      save() {},
    },
    options: { values: {}, errors: {}, selected: {}, get() {} },
    actions: { execute() {} },
    connection: { test() {} },
    media: {},
    provisioning: {
      supported: false,
      importToken() {},
      provision() {},
    },
    local: { get: () => undefined, set() {} },
    customNodes: {},
    formDrafts: {
      get: (key) => drafts.get(key),
      set: (key, value) => {
        drafts.set(key, value as Record<string, never>);
      },
      keys: () => [...drafts.keys()],
      clear: () => drafts.clear(),
    },
  };
}

describe('PluginUiNode', () => {
  test('renders a settings form inside cards and groups consecutive actions', () => {
    const node: PluginUiNodeContract = {
      type: 'card',
      title: { default: 'Twitch Live', i18key: '' },
      children: [
        { type: 'form', key: 'channelForm' },
        {
          type: 'button',
          label: { default: 'Connect', i18key: '' },
          variant: 'primary',
          action: { type: 'plugin-action', actionType: 'tiktools.twitch.connect' },
        },
        {
          type: 'button',
          label: { default: 'Disconnect', i18key: '' },
          variant: 'danger',
          action: { type: 'plugin-action', actionType: 'tiktools.twitch.disconnect' },
        },
        {
          type: 'button',
          label: { default: 'Refresh status', i18key: '' },
          action: { type: 'plugin-action', actionType: 'tiktools.twitch.status' },
        },
      ],
    };
    const wrapper = mountWithPlugins(PluginUiNode, {
      props: { node, context: context(), nodeKey: 's0' },
    });

    const input = wrapper.get('input');
    expect(input.attributes('placeholder')).toBe('shroud');
    expect(wrapper.text()).toContain('Twitch channel');

    const groups = wrapper.findAll('.plugin-node__actions');
    const actions = groups.find((group) => group.text().includes('Connect'));
    expect(actions).toBeDefined();
    expect(actions?.text()).toContain('Disconnect');
    expect(actions?.text()).toContain('Refresh status');
    expect(actions?.findAll('button')).toHaveLength(3);
  });
});
