<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor, PluginStatus } from '../../automation/behavior/types.ts';
import { pluginNavId, type PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { IconConnected, IconDice, IconLock, IconRadio } from '../components/icons.vue';
import { Alert, Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { TextField, PasswordField } from '../components/ui/fields/index.ts';
import { InfoTip } from '../components/ui/InfoTip.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { i18nText, t, type Locale } from '../i18n.ts';
import type { AppTab, ConnectionStatus } from '../types.ts';
import type { SuggestionItem } from '../components/autocomplete/types.ts';

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
  onUniqueIdChange: (val: string) => void;
  onCookieChange: (val: string) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  onReconnect: () => void;
  onPickLive: () => void;
  onSelectRecent: (username: string) => void;
  onTestConnection: (id: string) => void;
  onOpenPluginPage: (tab: AppTab) => void;
  onOpenPlugins: () => void;
};

/**
 * Unified Connections hub: TikTok LIVE (creator ComboBox + cookie) and
 * plugin server connections on one tab. TopNav owns the global connection
 * status and current creator, so this view renders no status badges.
 * Recent streamers live inside the creator autocomplete (open on focus,
 * filter while typing) instead of a second card.
 */
export const ConnectionsView = defineVueComponent<ConnectionsViewProps>(
  ['locale', 'uniqueId', 'cookie', 'status', 'recents', 'error', 'plugins', 'pluginPages', 'connections', 'onUniqueIdChange', 'onCookieChange', 'onConnect', 'onDisconnect', 'onReconnect', 'onPickLive', 'onSelectRecent', 'onTestConnection', 'onOpenPluginPage', 'onOpenPlugins'],
  (props) => {
  const showCookie = ref(Boolean(props.cookie));
  const testing = ref<Record<string, boolean>>({});
  const seenAt = ref<Record<string, number>>({});

  watch(() => props.connections, (next) => {
    for (const id of Object.keys(testing.value)) {
      if (!testing.value[id]) continue;
      const at = next[id]?.at ?? 0;
      if (at !== (seenAt.value[id] ?? 0)) {
        testing.value[id] = false;
        seenAt.value[id] = at;
      }
    }
  });

  const testServer = (id: string): void => {
    seenAt.value[id] = props.connections[id]?.at ?? 0;
    testing.value[id] = true;
    props.onTestConnection(id);
  };

  const configureTarget = (pluginId: string): AppTab | null => {
    const pages = props.pluginPages.filter((page) => page.pluginId === pluginId);
    if (pages.length === 0) return null;
    const withConnection = pages.find((page) => page.sections.some((section) => section.kind === 'connection'));
    const target = withConnection ?? pages[0];
    if (!target) return null;
    return pluginNavId(pluginId, target.id);
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
            <ul class="plg-list">
              {servers.map((plugin) => {
                const id = plugin.descriptor.id;
                const connection = props.connections[id];
                const busy = testing.value[id] === true;
                const dot = busy || !connection ? '' : connection.ok ? ' is-ok' : ' is-err';
                const detail = busy
                  ? t(locale, 'pluginTestingConnection')
                  : !connection
                    ? t(locale, 'pluginStatusNotConfigured')
                    : connection.ok
                      ? t(locale, 'pluginConnectedIn', { ms: connection.latencyMs })
                      : (connection.error || t(locale, 'pluginConnectionFailed'));
                const target = configureTarget(id);
                return (
                  <li key={id} class="plg-list__row">
                    <span class="plg-list__label" style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                      <span class={`plg-dot${dot}`} aria-hidden="true" />
                      <span style={{ fontWeight: 700 }}>{i18nText(locale, plugin.descriptor.name)}</span>
                      <span style={{ color: 'var(--text-muted)', fontSize: 12 }}>{detail}</span>
                    </span>
                    <span style={{ display: 'inline-flex', gap: 6 }}>
                      <button
                        type="button"
                        class="plg-btn plg-btn--sm"
                        disabled={busy}
                        onClick={() => testServer(id)}
                      >
                        {busy ? t(locale, 'pluginTestingConnection') : t(locale, 'connectionsTest')}
                      </button>
                      {target ? (
                        <button
                          type="button"
                          class="plg-btn plg-btn--sm"
                          onClick={() => props.onOpenPluginPage(target)}
                        >
                          {t(locale, 'connectionsConfigure')}
                        </button>
                      ) : null}
                    </span>
                  </li>
                );
              })}
            </ul>
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
