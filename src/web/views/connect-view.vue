<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import { IconConnected, IconDice, IconUsers } from '../components/icons.vue';
import { Alert, Badge, Card, Chip, ChipGroup, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { TextInput } from '../components/ui/TextInput.vue';
import { PasswordInput } from '../components/ui/PasswordInput.vue';
import { Page } from '../components/ui/Page.vue';
import { t, type Locale } from '../i18n.ts';
import type { ConnectionStatus } from '../types.ts';

type ConnectViewProps = {
  locale: Locale;
  uniqueId: string;
  cookie: string;
  status: ConnectionStatus;
  recents: string[];
  error: string;
  onUniqueIdChange: (val: string) => void;
  onCookieChange: (val: string) => void;
  onConnect: () => void;
  onPickLive: () => void;
  onSelectRecent: (username: string) => void;
};

export const ConnectView = defineVueComponent<ConnectViewProps>(
  ['locale', 'uniqueId', 'cookie', 'status', 'recents', 'error', 'onUniqueIdChange', 'onCookieChange', 'onConnect', 'onPickLive', 'onSelectRecent'],
  (props) => {
  const showCookie = ref(Boolean(props.cookie));

  return () => {
    const { locale, uniqueId, cookie, status, recents, error, onUniqueIdChange, onCookieChange, onConnect, onPickLive, onSelectRecent } = props;
    const isBusy = status === 'connecting' || status === 'retrying';
    const isLive = status === 'connected';
    const handleSubmit = (e: SubmitEvent) => {
      e.preventDefault();
      onConnect();
    };

    return (
      <Page>
        <div class="ui-cols-2">
        <Card title={t(locale, 'connectToLive')} subtitle={t(locale, 'setupLead')} icon={<IconConnected />}>
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
      </Page>
    );
  };
  },
);

export default ConnectView;
</script>
