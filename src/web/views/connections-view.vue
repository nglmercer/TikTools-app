<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor, PluginStatus } from '../../automation/behavior/types.ts';
import type { PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { IconConnected, IconDice, IconLock, IconRadio } from '../components/icons.vue';
import { Icon, readIconName } from '../components/icons/index.ts';
import { Alert, Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { TextField, PasswordField } from '../components/ui/fields/index.ts';
import { InfoTip } from '../components/ui/InfoTip.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { PluginConnectionCard } from '../components/plugin-connection-card.vue';
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

    return (
      <Page width="wide">
        <PageHeader
          title={t(locale, 'connectionsTitle')}
          icon={<IconConnected />}
        />

        <Card title={t(locale, 'connectToLive')} subtitle={t(locale, 'connectionsLiveLead')} icon={<IconRadio />}>
          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {isLive ? <Alert variant="info">{t(locale, 'live')} — {t(locale, 'disconnectToChangeCreator')}</Alert> : null}
            <TextField
              id="connect-creator"
              name="creator"
              value={uniqueId}
              onValueChange={onUniqueIdChange}
              options={recentOptions}
              onOptionPick={(picked) => onSelectRecent(picked)}
              label={t(locale, 'creatorHandle')}
              hint={t(locale, 'leadingAtOptional')}
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

            <div style={{ display: 'flex', gap: 12, marginTop: 12, alignItems: 'stretch' }}>
              <div style={{ display: 'flex', gap: 8, alignItems: 'center', flex: 'none' }}>
                {!showCookie.value ? (
                  <Button
                    variant="soft"
                    size="sm"
                    icon={<IconLock size={14} />}
                    tooltip={t(locale, 'guestCookieHint')}
                    onClick={() => (showCookie.value = true)}
                    disabled={isLive || isBusy}
                  >
                    {t(locale, 'authenticatedCookie')}
                  </Button>
                ) : null}
                <InfoTip text={t(locale, 'guestCookieHint')} position="top" />
              </div>
              <div style={{ display: 'flex', gap: 8, flex: '1 1 auto', minWidth: 0 }}>
                <Button type="submit" variant="primary" block loading={isBusy} disabled={isLive || !uniqueId.trim()}>
                  {isBusy ? t(locale, 'connecting') : isLive ? t(locale, 'live') : t(locale, 'connect')}
                </Button>
                <Button variant="cyan" tooltip={t(locale, 'pickLive')} disabled={isLive || isBusy} onClick={onPickLive} icon={<IconDice />} iconOnly />
              </div>
            </div>

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
          icon={<IconConnected />}
          action={servers.length ? <Badge>{servers.length}</Badge> : null}
        >
          {servers.length > 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              {servers.map((plugin) => {
                const id = plugin.descriptor.id;
                const page = connectionPageFor(id);
                // Reuse the connection page icon inline so the hidden
                // connection-only tab keeps its wayfinding without a
                // duplicate nav entry.
                const iconName = readIconName(page?.icon) ?? 'plugin';
                return (
                  <div key={id}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                      <span style={{ display: 'inline-flex', color: 'var(--tt-cyan)' }} aria-hidden="true">
                        <Icon name={iconName} size={16} />
                      </span>
                      <span style={{ fontWeight: 700 }}>{i18nText(locale, plugin.descriptor.name)}</span>
                    </div>
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
