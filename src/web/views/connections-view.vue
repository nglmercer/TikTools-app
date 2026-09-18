<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import type { PluginPageDescriptor, PluginStatus } from '../../automation/behavior/types.ts';
import { pluginNavId, type PluginConnectionState } from '../../automation/plugins/declarative.ts';
import { IconConnected, IconDice, IconRadio, IconUsers } from '../components/icons.vue';
import { Alert, Badge, Card, Chip, ChipGroup, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { TextInput } from '../components/ui/TextInput.vue';
import { PasswordInput } from '../components/ui/PasswordInput.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { i18nText, t, type Locale } from '../i18n.ts';
import type { AppTab, ConnectionStatus } from '../types.ts';

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
 * Unified Connections hub: TikTok LIVE (creator form + recent streamers)
 * and plugin server connections (status, test, configure) live on one tab
 * instead of two same-named "Connection" entries. Cards reuse the shared
 * Card/Badge/Chip primitives and the plugin status copy so both connection
 * kinds read as one experience.
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
    const statusKey = status === 'connected'
      ? 'live'
      : status === 'connecting' || status === 'retrying'
        ? 'connecting'
        : status === 'error'
          ? 'needsAttention'
          : 'disconnected';
    const handleSubmit = (e: SubmitEvent) => {
      e.preventDefault();
      onConnect();
    };
    const servers = props.plugins.filter(
      (plugin) => plugin.installed && plugin.enabled && plugin.descriptor.hasConnectionProbe,
    );

    return (
      <Page width="wide">
        <PageHeader
          title={t(locale, 'connectionsTitle')}
          icon={<IconConnected />}
          meta={(
            <span style={{ display: 'inline-flex', gap: 6, alignItems: 'center' }}>
              <Badge>{t(locale, statusKey)}</Badge>
              {props.uniqueId ? <Badge>@{props.uniqueId.replace(/^@/, '')}</Badge> : null}
            </span>
          )}
        />

        <div class="ui-cols-2">
          <Card title={t(locale, 'connectToLive')} subtitle={t(locale, 'connectionsLiveLead')} icon={<IconRadio />}>
            <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {isLive ? <Alert variant="info">{t(locale, 'live')} — {t(locale, 'disconnectToChangeCreator')}</Alert> : null}
              <TextInput
                id="connect-creator"
                name="creator"
                value={uniqueId}
                onValueChange={onUniqueIdChange}
                label={t(locale, 'creatorHandle')}
                hint={t(locale, 'leadingAtOptional')}
                prefix="@"
                required
                disabled={isLive || isBusy}
                onEnter={onConnect}
              />

              {showCookie.value ? (
                <PasswordInput
                  id="connect-cookie"
                  name="cookie"
                  value={cookie}
                  onValueChange={onCookieChange}
                  label={`${t(locale, 'authenticatedCookie')} ${t(locale, 'optional')}`}
                  hint={t(locale, 'guestCookieHint')}
                  disabled={isLive || isBusy}
                  autoComplete="off"
                  locale={locale}
                />
              ) : (
                <div style={{ marginBottom: 4 }}>
                  <Button variant="soft" size="sm" onClick={() => (showCookie.value = true)} disabled={isLive || isBusy}>
                    + {t(locale, 'authenticatedCookie')}
                  </Button>
                </div>
              )}

              {error ? <Alert variant="danger">{error}</Alert> : null}

              <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
                <Button type="submit" variant="primary" block loading={isBusy} disabled={isLive || !uniqueId.trim()}>
                  {isBusy ? t(locale, 'connecting') : isLive ? t(locale, 'live') : t(locale, 'connect')}
                </Button>
                <Button variant="cyan" tooltip={t(locale, 'pickLive')} disabled={isLive || isBusy} onClick={onPickLive} icon={<IconDice />} iconOnly />
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

          <Card title={t(locale, 'recentStreamers')} icon={<IconUsers />} action={recents.length ? <Badge>{recents.length}</Badge> : null}>
            {recents.length > 0 ? (
              <ChipGroup>
                {recents.map((creator) => (
                  <Chip key={creator} onClick={() => onSelectRecent(creator)}>
                    @{creator}
                  </Chip>
                ))}
              </ChipGroup>
            ) : (
              <EmptyState title={t(locale, 'noRecents')} description="" />
            )}
          </Card>
        </div>

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
