import { describe, expect, test } from 'bun:test';

import type { PluginStatus } from '../../automation/behavior/types.ts';
import {
  buildWidgetObsUrl,
  GATEWAY_DEFAULT_PORT,
  gatewayPluginStatus,
  GATEWAY_PLUGIN_ID,
  parseGatewayWidgetConfig,
} from './widgets.ts';

function pluginStatus(id: string, overrides: Partial<PluginStatus> = {}): PluginStatus {
  return {
    descriptor: {
      id,
      name: { default: id, i18key: `${id}.name` },
      version: '1.0.0',
      description: { default: '', i18key: `${id}.description` },
      dependency: { default: '', i18key: `${id}.dependency` },
      permissions: [],
      actionTypeIds: [],
      eventTypeIds: [],
    },
    installed: true,
    enabled: true,
    available: true,
    ...overrides,
  };
}

describe('widgets settings', () => {
  test('parses gateway config with loopback defaults', () => {
    expect(parseGatewayWidgetConfig(undefined)).toEqual({
      host: '127.0.0.1',
      port: GATEWAY_DEFAULT_PORT,
      token: null,
    });
    expect(parseGatewayWidgetConfig({ port: 18000, token: 'ttk_abc' })).toEqual({
      host: '127.0.0.1',
      port: 18000,
      token: 'ttk_abc',
    });
  });

  test('rejects invalid ports and blank tokens', () => {
    expect(parseGatewayWidgetConfig({ port: 99999, token: '  ' })).toEqual({
      host: '127.0.0.1',
      port: GATEWAY_DEFAULT_PORT,
      token: null,
    });
    expect(parseGatewayWidgetConfig({ port: '17452' })).toEqual({
      host: '127.0.0.1',
      port: GATEWAY_DEFAULT_PORT,
      token: null,
    });
  });

  test('builds OBS urls with the token in the fragment', () => {
    const config = parseGatewayWidgetConfig({ port: 17452, token: 'ttk_abc' });
    expect(buildWidgetObsUrl(config, 'follow')).toBe(
      'http://127.0.0.1:17452/widgets/follow/#token=ttk_abc',
    );
    expect(buildWidgetObsUrl(config, 'gift')).toBe(
      'http://127.0.0.1:17452/widgets/gift/#token=ttk_abc',
    );
    expect(buildWidgetObsUrl({ host: '127.0.0.1', port: 17452, token: null }, 'follow')).toBe(
      'http://127.0.0.1:17452/widgets/follow/',
    );
  });

  test('finds the gateway plugin status by id', () => {
    const gateway = pluginStatus(GATEWAY_PLUGIN_ID);
    expect(gatewayPluginStatus([pluginStatus('other'), gateway])).toBe(gateway);
    expect(gatewayPluginStatus([pluginStatus('other')])).toBeUndefined();
  });
});
