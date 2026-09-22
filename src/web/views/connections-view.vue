<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor, PluginStatus } from '../../automation/behavior/types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { IconConnected, IconDice, IconLock, IconRadio, IconServer } from '../components/icons.vue';
import { Icon, type IconName } from '../components/icons/index.ts';
import { Alert, Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { TextField, PasswordField } from '../components/ui/fields/index.ts';
import { InfoTip } from '../components/ui/InfoTip.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { Tooltip } from '../components/ui/Tooltip.vue';
import { PluginConnectionCard } from '../components/plugin-connection-card.vue';
import { connectionIconFor } from '../components/plugin-cards.ts';
import { i18nText, t, type Locale } from '../i18n.ts';
import type { ConnectionStatus, PluginSettingsState } from '../types.ts';
import type { SuggestionItem } from '../components/autocomplete/types.ts';
import type {
  ActionOptionItem,
  OpenMediaPicker,
  PluginSettingValues,
} from '../../shared/messages.ts';

type ConnectionsViewProps = {
  locale: Locale;
  uniqueId: string;
  cookie: string;
  status: ConnectionStatus;
  recents: string[];
  error: string;
  plugins: PluginStatus[];
  pluginPages: PluginPageDescriptor[];
  connections: Record<string, PluginConnectionState>;
  pluginSettings: Record<string, PluginSettingsState | undefined>;
  actionOptions: Record<string, ActionOptionItem[]>;
  provisionStates: Record<string, { working: boolean; ok: boolean; message: string } | undefined>;
  onUniqueIdChange: (val: string) => void;
  onCookieChange: (val: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  onReconnect: () => void;
  onPickLive: () => void;
  onSelectRecent: (username: string) => void;
  onTestConnection: (id: string) => void;
  onGetSettings: (id: string) => void;
  onSaveSettings: (id: string, values: PluginSettingValues) => void;
  onGetActionOptions: (source: string) => void;
  onOpenMediaPicker?: OpenMediaPicker;
  onProvisionToken?: (id: string, username: string, password: string) => void;
  onOpenPlugins: () => void;
};

/**
 * Unified Connections hub: TikTok LIVE (creator ComboBox + cookie) and
 * every plugin server connection on one tab. TopNav owns the global
 * connection status and current creator, so this view renders no status
 * badges. Recent streamers live inside the creator autocomplete
 * (open on focus, filter while typing) instead of a second card.
 *
 * Server connections render the shared PluginConnectionCard inline — the
 * same component as the plugin `connection` page section. Connection-only
 * plugin pages stay out of the nav rail (see isConnectionOnlyPage) so
 * each server plugin doesn't mint a duplicate minimal tab; their page
 * icon is reused here instead.
 */
export const ConnectionsView = defineVueComponent<ConnectionsViewProps>(
  ['locale', 'uniqueId', 'cookie', 'status', 'recents', 'error', 'plugins', 'pluginPages', 'connections', 'pluginSettings', 'actionOptions', 'provisionStates', 'onUniqueIdChange', 'onCookieChange', 'onConnect', 'onDisconnect', 'onReconnect', 'onPickLive', 'onSelectRecent', 'onTestConnection', 'onGetSettings', 'onSaveSettings', 'onGetActionOptions', 'onOpenMediaPicker', 'onProvisionToken', 'onOpenPlugins'],
  (props) => {
  const showCookie = ref(Boolean(props.cookie));
  // Manual per-server overrides; untouched servers stay compact. The latest
  // probe detail remains available through the header tooltip, and clicking
  // the row opens the full settings form.
  const openServers = ref<Record<string, boolean>>({});
  const isServerOpen = (id: string): boolean => openServers.value[id] ?? false;

  const connectionPageFor = (pluginId: string): PluginPageDescriptor | undefined => {
    const pages = props.pluginPages.filter((page) => page.pluginId === pluginId);
    return pages.find((page) => page.sections.some((section) => section.kind === 'connection')) ?? pages[0];
  };

  return () => {
    const { locale, uniqueId, cookie, status, recents, error, onUniqueIdChange, onCookieChange, onConnect, onDisconnect, onReconnect, onPickLive, onSelectRecent } = props;
    const isBusy = status === 'connecting' || status === 'retrying';
    const isLive = status === 'connected';
    const handleSubmit = (e: SubmitEvent) => {
      e.preventDefault();
      onConnect();
    };
    const servers = props.plugins.filter(
      (plugin) => plugin.installed && plugin.enabled && plugin.descriptor.hasConnectionProbe,
    );
    const recentOptions: SuggestionItem[] = recents.map((creator) => ({
      value: creator.replace(/^@/, ''),
      label: `@${creator.replace(/^@/, '')}`,
      kind: 'path' as const,
      icon: 'users' as const,
    }));
    const usedConnectionIcons = new Set<IconName>();

    return (
      <Page width="wide">
        <PageHeader
          title={t(locale, 'connectionsTitle')}
          icon={<IconConnected />}
        />

        <Card title={t(locale, 'connectToLive')} subtitle={t(locale, 'connectionsLiveLead')} icon={<IconRadio />}>
          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <TextField
              id="connect-creator"
              name="creator"
              value={uniqueId}
              onValueChange={onUniqueIdChange}
              options={recentOptions}
              onOptionPick={(picked) => onSelectRecent(picked)}
              label={t(locale, 'creatorHandle')}
              hint={t(locale, isLive ? 'disconnectToChangeCreator' : 'leadingAtOptional')}
              prefix="@"
              placeholder={t(locale, 'usernamePlaceholder')}
              required
              disabled={isLive || isBusy}
              onEnter={onConnect}
              locale={locale}
              autoComplete="off"
            />

            {showCookie.value ? (
              <PasswordField
                id="connect-cookie"
                name="cookie"
                value={cookie}
                onValueChange={onCookieChange}
                label={`${t(locale, 'authenticatedCookie')} ${t(locale, 'optional')}`}
                hint={t(locale, 'guestCookieHint')}
                placeholder={t(locale, 'cookiePlaceholder')}
                disabled={isLive || isBusy}
                autoComplete="off"
                locale={locale}
              />
            ) : null}

            {error ? <Alert variant="danger">{error}</Alert> : null}

            {!isLive ? (
              <div style={{ display: 'flex', gap: 12, marginTop: 12, alignItems: 'stretch' }}>
                <div style={{ display: 'flex', gap: 8, alignItems: 'center', flex: 'none' }}>
                  {!showCookie.value ? (
                    <Button
                      variant="soft"
                      size="sm"
                      icon={<IconLock size={14} />}
                      tooltip={t(locale, 'guestCookieHint')}
                      onClick={() => (showCookie.value = true)}
                      disabled={isBusy}
                    >
                      {t(locale, 'authenticatedCookie')}
                    </Button>
                  ) : null}
                  <InfoTip text={t(locale, 'guestCookieHint')} position="top" />
                </div>
                <div style={{ display: 'flex', gap: 8, flex: '1 1 auto', minWidth: 0 }}>
                  <Button type="submit" variant="primary" block loading={isBusy} disabled={!uniqueId.trim()}>
                    {isBusy ? t(locale, 'connecting') : t(locale, 'connect')}
                  </Button>
                  <Button variant="cyan" tooltip={t(locale, 'pickLive')} disabled={isBusy} onClick={onPickLive} icon={<IconDice />} iconOnly />
                </div>
              </div>
            ) : null}

            {isLive || status === 'error' ? (
              <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
                {isLive ? (
                  <Button variant="danger" block onClick={onDisconnect}>
                    {t(locale, 'disconnect')}
                  </Button>
                ) : null}
                <Button variant="soft" block onClick={onReconnect}>
                  {t(locale, 'reconnect')}
                </Button>
              </div>
            ) : null}
          </form>
        </Card>

        <Card
          title={t(locale, 'connectionsServers')}
          subtitle={t(locale, 'connectionsServersLead')}
          icon={<IconServer />}
          action={servers.length ? <Badge>{servers.length}</Badge> : null}
        >
          {servers.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              {servers.map((plugin) => {
                const id = plugin.descriptor.id;
                const page = connectionPageFor(id);
                // Prefer the plugin's own icon, then its hidden connection
                // page icon, with a stable unique fallback for this list.
                const iconName = connectionIconFor(plugin.descriptor, page?.icon, usedConnectionIcons);
                usedConnectionIcons.add(iconName);
                const conn = props.connections[id];
                const pluginName = i18nText(locale, plugin.descriptor.name);
                const connectionHint = conn
                  ? conn.ok
                    ? t(locale, 'pluginConnectedIn', { ms: conn.latencyMs })
                    : (conn.error || t(locale, 'pluginConnectionFailed'))
                  : t(locale, 'pluginStatusNotConfigured');
                const open = isServerOpen(id);
                // The card stays mounted while collapsed so its auto-probe,
                // draft, and autosave state survive; collapse is purely visual.
                return (
                  <div key={id} class={`srv-server${open ? ' is-open' : ''}`}>
                    <Tooltip text={connectionHint} position="top" wide>
                      <button
                        type="button"
                        class="srv-server__head"
                        aria-label={`${pluginName}: ${connectionHint}`}
                        aria-expanded={open ? 'true' : 'false'}
                        aria-controls={`srv-body-${id}`}
                        onClick={() => {
                          openServers.value = { ...openServers.value, [id]: !open };
                        }}
                      >
                        <span class="srv-server__icon" aria-hidden="true">
                          <Icon name={iconName} size={15} />
                        </span>
                        <span class="srv-server__name">{pluginName}</span>
                        {conn ? <span class={`plg-dot${conn.ok ? ' is-ok' : ' is-err'}`} aria-hidden="true" /> : null}
                        {conn && !conn.ok ? (
                          <span class="srv-server__detail" aria-hidden="true">
                            <Icon name="info" size={13} />
                          </span>
                        ) : null}
                        <span class="srv-server__chevron" aria-hidden="true">
                          <Icon name="chevron-right" size={15} />
                        </span>
                      </button>
                    </Tooltip>
                    <div id={`srv-body-${id}`} class="srv-server__body">
                      <PluginConnectionCard
                        inline
                        locale={locale}
                        pluginId={id}
                        pluginName={i18nText(locale, plugin.descriptor.name)}
                        settingsState={props.pluginSettings[id]}
                        connection={props.connections[id]}
                        actionOptions={props.actionOptions}
                        onGetSettings={props.onGetSettings}
                        onSaveSettings={props.onSaveSettings}
                        onGetActionOptions={props.onGetActionOptions}
                        onTestConnection={props.onTestConnection}
                        onOpenMediaPicker={props.onOpenMediaPicker}
                        supportsProvisioning={plugin.descriptor.supportsTokenProvisioning}
                        provisionState={props.provisionStates[id]}
                        onProvisionToken={props.onProvisionToken}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <EmptyState
              title={t(locale, 'connectionsNoServers')}
              description={t(locale, 'connectionsNoServersHint')}
              action={(
                <Button variant="soft" size="sm" onClick={props.onOpenPlugins}>
                  {t(locale, 'connectionsOpenPlugins')}
                </Button>
              )}
            />
          )}
        </Card>
      </Page>
    );
  };
  },
);

export default ConnectionsView;
</script>
