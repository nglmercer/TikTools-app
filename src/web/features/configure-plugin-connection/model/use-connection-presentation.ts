import { computed, markRaw, type Component, type Ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonObject } from '../../../../automation/types.ts';
import { connectionSummaryRows, isLoopbackUrl, secretSettingKeys } from '@/entities/plugin/lib/connection-settings.ts';
import { fieldOptionsFor } from '@/entities/plugin/lib/plugin-options.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import IconClock from '@/shared/ui/icon/icons/IconClock.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconHash from '@/shared/ui/icon/icons/IconHash.vue';
import IconKey from '@/shared/ui/icon/icons/IconKey.vue';
import IconMapPin from '@/shared/ui/icon/icons/IconMapPin.vue';
import IconUser from '@/shared/ui/icon/icons/IconUser.vue';
import type { PluginConnectionEditorProps } from './connection-editor-types.ts';
import type { ConnectionAutosave } from './use-connection-autosave.ts';

export function useConnectionPresentation(
  props: Readonly<PluginConnectionEditorProps>,
  autosave: ConnectionAutosave,
  context: {
    guided: Readonly<Ref<boolean>>;
    advanced: Readonly<Ref<boolean>>;
    needsAuthorization: Readonly<Ref<boolean | undefined>>;
  },
) {
  const { t } = useI18n();
  const { guided, advanced, needsAuthorization } = context;
  const { testing, values, url, urlKey, urlInvalid, dirty } = autosave;
  const visibleSchema = computed(() => {
    const schema = props.settings?.schema;
    if (!schema || !guided.value || advanced.value) return schema ?? {};
    const properties = schema.properties as JsonObject | undefined;
    return { ...schema, properties: Object.fromEntries(Object.entries(properties ?? {}).filter(([key]) => key !== 'apiToken')) };
  });
  const fieldOptions = computed(() => fieldOptionsFor(props.settings?.uiHints, props.actionOptions));
  const fieldErrors = computed(() => (urlInvalid.value && urlKey.value ? { [urlKey.value]: t('plugins.connection.invalidUrl') } : {}));
  const summaryRows = computed(() =>
    connectionSummaryRows(values.value, props.settings?.schema, props.settings?.uiHints, urlKey.value, localizedText),
  );
  const localHint = computed(() => (url.value && isLoopbackUrl(url.value) ? t('plugins.connection.localNote') : t('plugins.connection.hint')));
  const tone = computed(() => {
    if (dirty.value || testing.value || !props.connection) return 'idle';
    return props.connection.ok ? 'ok' : 'error';
  });

  const statusText = computed(() => {
    const connection = props.connection;
    if (dirty.value) return t('plugins.connection.testToVerify');
    if (testing.value) return t('plugins.connection.testing');
    if (!connection) return t('connections.servers.notConfigured');
    if (guided.value && needsAuthorization.value) return t('plugins.connection.authorize');
    if (guided.value && connection.ok) return t('plugins.connection.connected');
    if (connection.reachable && connection.gameplayReady === false) return t('connections.servers.apiNotReady');
    return connection.ok ? t('connections.servers.connectedIn', { ms: connection.latencyMs }) : t('connections.servers.failed');
  });

  /** Slim failure line for the sectioned layout; the panel header already shows the status. */
  const failureText = computed(() => {
    const connection = props.connection;
    if (dirty.value || testing.value || !connection || connection.ok) return '';
    if (connection.details) return connection.details;
    if (connection.error) return connection.error;
    if (guided.value && !connection.reachable) return t('plugins.connection.openGuided', { name: props.pluginName });
    return statusText.value;
  });

  /** Keys that describe how to reach the service; everything else is a gameplay default. */
  const connectionKeys = computed(() => new Set([urlKey.value, ...secretSettingKeys(props.settings?.schema, props.settings?.uiHints)]));
  function schemaSubset(schema: JsonObject, keep: (key: string) => boolean): JsonObject {
    const properties = schema.properties;
    if (!properties || typeof properties !== 'object' || Array.isArray(properties)) return schema;
    return { ...schema, properties: Object.fromEntries(Object.entries(properties as JsonObject).filter(([key]) => keep(key))) };
  }
  const connectionSchema = computed(() => schemaSubset(visibleSchema.value, (key) => connectionKeys.value.has(key)));
  const defaultsSchema = computed(() => schemaSubset(visibleSchema.value, (key) => !connectionKeys.value.has(key)));
  const hasDefaults = computed(() => {
    const properties = defaultsSchema.value.properties;
    return Boolean(properties && typeof properties === 'object' && !Array.isArray(properties) && Object.keys(properties).length > 0);
  });
  const fieldIcons = markRaw<Readonly<Record<string, Component>>>({
    apiToken: IconKey,
    serverUrl: IconGlobe,
    playerId: IconUser,
    positionMode: IconMapPin,
    quantity: IconHash,
    lifetimeSeconds: IconClock,
  });

  return { visibleSchema, fieldOptions, fieldErrors, summaryRows, localHint, tone, statusText, failureText, connectionSchema, defaultsSchema, hasDefaults, fieldIcons };
}
export type ConnectionPresentation = ReturnType<typeof useConnectionPresentation>;
