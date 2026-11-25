import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, test, vi } from 'vitest';

import type { PluginUiNode as PluginUiNodeContract } from '../../../../../src/plugin-ui/index.ts';
import type { PluginUiContext } from '@/entities/plugin/model/plugin-ui-context.ts';
import type { PluginActionOutcome } from '@/entities/plugin/model/plugins-state.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginUiNode from '@/widgets/plugin-page/ui/PluginUiNode.vue';

enableAutoUnmount(afterEach);

function outcome(actionType: string): PluginActionOutcome {
  return {
    actionType,
    ok: true,
    summary: 'Twitch: running (Connected to #room)',
    logs: [],
    durationMs: 1,
    result: {
      phase: 'connected',
      workerRunning: true,
      channel: 'room',
      message: 'Connected to #room',
      updatedAt: 1700000000000,
    },
  };
}

function context(execute: (actionType: string) => Promise<ReturnType<typeof outcome>> = (actionType) => Promise.resolve(outcome(actionType))): PluginUiContext {
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
    actions: { execute },
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

  test('renders structured action status from the plugin outcome', async () => {
    const node: PluginUiNodeContract = {
      type: 'action-status',
      action: { type: 'plugin-action', actionType: 'tiktools.twitch.status' },
      pollMs: 60_000,
    };
    const wrapper = mountWithPlugins(PluginUiNode, {
      props: { node, context: context(), nodeKey: 's0' },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('Connected to #room');
    expect(wrapper.text()).toContain('room');
    expect(wrapper.text()).toContain('connected');
    expect(wrapper.text()).toContain('running');
  });


  test('disables incompatible actions from published local state', () => {
    const ctx = context();
    ctx.local.get = (name) => (name === 'connectionPhase' ? 'connected' : undefined);
    const node: PluginUiNodeContract = {
      type: 'button',
      label: { default: 'Connect', i18key: '' },
      action: { type: 'plugin-action', actionType: 'tiktools.twitch.connect' },
      disabledWhen: { bind: 'local.connectionPhase', values: ['connected', 'connecting'] },
    };
    const wrapper = mountWithPlugins(PluginUiNode, {
      props: { node, context: ctx, nodeKey: 's0' },
    });
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
  });

  test('shows inline pending and persistent error feedback for actions', async () => {
    let settle!: (value: ReturnType<typeof outcome>) => void;
    const execute = vi.fn(
      (actionType: string) =>
        new Promise<ReturnType<typeof outcome>>((resolve) => {
          settle = resolve;
          void actionType;
        }),
    );
    const node: PluginUiNodeContract = {
      type: 'button',
      label: { default: 'Connect', i18key: '' },
      action: { type: 'plugin-action', actionType: 'tiktools.twitch.connect' },
    };
    const wrapper = mountWithPlugins(PluginUiNode, {
      props: { node, context: context(execute), nodeKey: 's0' },
    });

    await wrapper.get('button').trigger('click');
    expect(wrapper.get('button').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('Working…');

    settle({ ...outcome('tiktools.twitch.connect'), ok: false, error: 'channel is required' });
    await flushPromises();
    expect(wrapper.get('button').attributes('disabled')).toBeUndefined();
    expect(wrapper.text()).toContain('channel is required');
  });
});
