import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { effectScope, nextTick, ref } from 'vue';
import type { PluginConnectionState } from '../../../../../../src/automation/plugins/declarative.ts';
import type { PluginSettingValues } from '../../../../../../src/shared/messages.ts';
import { AUTOSAVE_CONFIRM_TIMEOUT_MS, AUTOSAVE_DEBOUNCE_MS } from '@/entities/plugin/lib/connection-settings.ts';
import type { PluginSettingsState } from '@/types.ts';
import { useConnectionAutosave } from '../../../../../../src/web/features/configure-plugin-connection/model/use-connection-autosave.ts';

const schema = {
  properties: {
    url: { type: 'string', format: 'uri', default: 'http://127.0.0.1:8080' },
    token: { type: 'string', secret: true },
  },
};

function setup(values: PluginSettingValues = {}) {
  const scope = effectScope();
  const pluginId = ref('srv');
  const settings = ref<PluginSettingsState | undefined>({ schema, values } as unknown as PluginSettingsState);
  const connection = ref<PluginConnectionState | undefined>(undefined);
  const saved: PluginSettingValues[] = [];
  const autosave = scope.run(() =>
    useConnectionAutosave({ pluginId, settings, connection, save: (next) => saved.push(next) }),
  )!;
  return { scope, pluginId, settings, connection, saved, autosave };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('useConnectionAutosave', () => {
  it('fills defaults and validates the server url', () => {
    const { autosave } = setup();
    expect(autosave.url.value).toBe('http://127.0.0.1:8080');
    expect(autosave.urlInvalid.value).toBe(false);
    autosave.edit({ url: 'ftp://nope' });
    expect(autosave.urlInvalid.value).toBe(true);
  });

  it('debounces edits into a single save', () => {
    const { autosave, saved } = setup();
    autosave.edit({ url: 'http://a' });
    autosave.edit({ url: 'http://b' });
    expect(saved).toHaveLength(0);
    vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS);
    expect(saved).toEqual([{ url: 'http://b' }]);
    expect(autosave.saveState.value).toBe('saving');
  });

  it('confirms the save when the host echoes it', async () => {
    const { autosave, settings } = setup();
    autosave.edit({ url: 'http://b' });
    autosave.flush();
    settings.value = { ...settings.value!, values: { url: 'http://b' } };
    await nextTick();
    expect(autosave.saveState.value).toBe('saved');
    expect(autosave.dirty.value).toBe(false);
  });

  it('reports an error when no echo arrives', () => {
    const { autosave } = setup();
    autosave.edit({ url: 'http://b' });
    autosave.flush();
    vi.advanceTimersByTime(AUTOSAVE_CONFIRM_TIMEOUT_MS);
    expect(autosave.saveState.value).toBe('error');
  });

  it('skips saving an unchanged payload twice', () => {
    const { autosave, saved } = setup();
    autosave.edit({ url: 'http://b' });
    autosave.flush();
    autosave.flush();
    expect(saved).toHaveLength(1);
  });

  it('resets the draft when the plugin changes', async () => {
    const { autosave, pluginId } = setup();
    autosave.edit({ url: 'http://b' });
    pluginId.value = 'other';
    await nextTick();
    expect(autosave.draft.value).toBeNull();
    expect(autosave.saveState.value).toBe('idle');
  });

  it('leaves edit mode after a successful probe', async () => {
    const { autosave, connection } = setup();
    autosave.editing.value = true;
    autosave.testing.value = true;
    connection.value = { ok: true, at: 1 } as PluginConnectionState;
    await nextTick();
    expect(autosave.testing.value).toBe(false);
    expect(autosave.editing.value).toBe(false);
  });
});
