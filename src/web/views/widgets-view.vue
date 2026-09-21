<script lang="tsx">
import { computed, onMounted, ref, watch } from 'vue';
import type { PluginStatus } from '../../automation/behavior/types.ts';
import { IconCheck, IconCopy, IconFollow, IconGift, IconRefresh } from '../components/icons.vue';
import { Button } from '../components/ui/Button.vue';
import { Card } from '../components/ui/Card.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { TextInput } from '../components/ui/TextInput.vue';
import {
  buildWidgetObsUrl,
  copyTextToClipboard,
  GATEWAY_PLUGIN_ID,
  gatewayPluginStatus,
  parseGatewayWidgetConfig,
  probeGatewayHealth,
  type GatewayHealth,
  type WidgetKind,
} from '../features/widgets.ts';
import { t, type Locale } from '../i18n.ts';
import type { PluginSettingsState } from '../types.ts';
import { defineVueComponent } from '../vue/component.ts';

type WidgetsViewProps = {
  locale: Locale;
  plugins: PluginStatus[];
  settings: Record<string, PluginSettingsState | undefined>;
  onGetSettings: (id: string) => void;
};

type GatewayState = 'missing' | 'disabled' | 'stopped' | 'running';

function renderWidgetCard(args: {
  locale: Locale;
  widget: WidgetKind;
  title: string;
  description: string;
  icon: typeof IconGift;
  obsUrl: string;
  hasToken: boolean;
  previewUrl: string;
  previewKey: number;
  copied: boolean;
  onCopy: () => void;
  onReplay: () => void;
}) {
  const { locale } = args;
  return (
    <Card title={args.title} icon={<args.icon />}>
      <p class="widgets-description">{args.description}</p>
      <div class="widgets-field-label">{t(locale, 'widgetsObsUrl')}</div>
      <div class="widgets-url-row">
        <TextInput
          value={args.obsUrl}
          onValueChange={() => {}}
          readonly
          spellCheck={false}
          autoComplete="off"
        />
        <Button
          variant="soft"
          size="md"
          icon={args.copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
          onClick={args.onCopy}
        >
          {args.copied ? t(locale, 'widgetsCopied') : t(locale, 'widgetsCopyUrl')}
        </Button>
      </div>
      {!args.hasToken ? <p class="widgets-hint">{t(locale, 'widgetsNeedToken')}</p> : null}
      <p class="widgets-token-note">{t(locale, 'widgetsTokenNote')}</p>
      <div class="widgets-field-label">{t(locale, 'widgetsPreview')}</div>
      <div class="widgets-preview-frame">
        <iframe
          key={`${args.widget}-${args.previewKey}`}
          src={args.previewUrl}
          title={args.title}
          sandbox="allow-scripts"
        />
      </div>
      <div class="widgets-preview-actions">
        <Button variant="ghost" size="sm" icon={<IconRefresh size={14} />} onClick={args.onReplay}>
          {t(locale, 'widgetsReplay')}
        </Button>
      </div>
    </Card>
  );
}

/**
 * Widgets: OBS Browser Source URLs for the Follow and Gift alerts. URLs are
 * derived from the Event Gateway plugin settings (port + token) so users
 * never type them by hand. Previews run the widget in local demo mode, which
 * needs no gateway connection.
 */
export const WidgetsView = defineVueComponent<WidgetsViewProps>(
  ['locale', 'plugins', 'settings', 'onGetSettings'],
  (props) => {
    const health = ref<GatewayHealth>('unknown');
    const copied = ref<WidgetKind | null>(null);
    const previewNonce = ref(0);
    let copiedTimer: ReturnType<typeof setTimeout> | undefined;

    const gateway = computed(() => gatewayPluginStatus(props.plugins));
    const config = computed(() => parseGatewayWidgetConfig(props.settings[GATEWAY_PLUGIN_ID]?.values));

    const gatewayState = computed<GatewayState>(() => {
      const status = gateway.value;
      if (!status || !status.installed) return 'missing';
      if (!status.enabled || !status.available) return 'disabled';
      if (status.running === false) return 'stopped';
      return 'running';
    });

    const refresh = (): void => {
      props.onGetSettings(GATEWAY_PLUGIN_ID);
    };

    const probe = async (): Promise<void> => {
      health.value = 'checking';
      health.value = await probeGatewayHealth(config.value.host, config.value.port);
    };

    onMounted(() => {
      if (!props.settings[GATEWAY_PLUGIN_ID]) refresh();
    });

    watch(
      () => props.settings[GATEWAY_PLUGIN_ID],
      (state) => {
        if (state) void probe();
      },
      { immediate: true },
    );

    const copyUrl = (widget: WidgetKind): void => {
      const url = buildWidgetObsUrl(config.value, widget);
      void copyTextToClipboard(url).then((ok) => {
        if (!ok) return;
        copied.value = widget;
        if (copiedTimer) clearTimeout(copiedTimer);
        copiedTimer = setTimeout(() => {
          copied.value = null;
        }, 2000);
      });
    };

    const statusText = computed(() => {
      const state = gatewayState.value;
      if (state === 'missing') return t(props.locale, 'widgetsStatusMissing');
      if (state === 'disabled') return t(props.locale, 'widgetsStatusDisabled');
      if (state === 'stopped') return t(props.locale, 'widgetsStatusStopped');
      return t(props.locale, 'widgetsStatusRunning', { port: config.value.port });
    });

    const healthText = computed(() => {
      const current = health.value;
      if (current === 'ok') return t(props.locale, 'widgetsHealthOk');
      if (current === 'checking') return t(props.locale, 'widgetsHealthChecking');
      if (current === 'unreachable') return t(props.locale, 'widgetsHealthUnreachable');
      return t(props.locale, 'widgetsHealthUnknown');
    });

    return () => {
      const locale = props.locale;
      const followUrl = buildWidgetObsUrl(config.value, 'follow');
      const giftUrl = buildWidgetObsUrl(config.value, 'gift');
      // Previews use tokenless demo mode: the widget plays one synthetic
      // alert locally instead of connecting to the gateway.
      const followPreview = `http://${config.value.host}:${config.value.port}/widgets/follow/#demo=follow`;
      const giftPreview = `http://${config.value.host}:${config.value.port}/widgets/gift/#demo=combo`;
      const healthy = gatewayState.value === 'running' && health.value === 'ok';
      return (
        <Page width="wide">
          <PageHeader
            title={t(locale, 'tabWidgets')}
            subtitle={t(locale, 'widgetsSubtitle')}
            icon={<IconGift />}
            action={
              <Button variant="soft" size="md" icon={<IconRefresh size={14} />} onClick={refresh}>
                {t(locale, 'widgetsRefresh')}
              </Button>
            }
          />
          <Card title={t(locale, 'widgetsGateway')} icon={<IconGift />}>
            <div class="widgets-status-row">
              <span class={`ui-badge ${healthy ? 'ui-badge--cyan' : ''}`}>{statusText.value}</span>
              <span class="widgets-health">
                {t(locale, 'widgetsHealth')}: {healthText.value}
              </span>
            </div>
            {health.value === 'unreachable' ? (
              <p class="widgets-hint">{t(locale, 'widgetsHealthHint')}</p>
            ) : null}
          </Card>
          <div class="ui-cols-2">
            {renderWidgetCard({
              locale,
              widget: 'follow',
              title: t(locale, 'widgetsFollowTitle'),
              description: t(locale, 'widgetsFollowDescription'),
              icon: IconFollow,
              obsUrl: followUrl,
              hasToken: config.value.token !== null,
              previewUrl: followPreview,
              previewKey: previewNonce.value,
              copied: copied.value === 'follow',
              onCopy: () => copyUrl('follow'),
              onReplay: () => {
                previewNonce.value += 1;
              },
            })}
            {renderWidgetCard({
              locale,
              widget: 'gift',
              title: t(locale, 'widgetsGiftTitle'),
              description: t(locale, 'widgetsGiftDescription'),
              icon: IconGift,
              obsUrl: giftUrl,
              hasToken: config.value.token !== null,
              previewUrl: giftPreview,
              previewKey: previewNonce.value,
              copied: copied.value === 'gift',
              onCopy: () => copyUrl('gift'),
              onReplay: () => {
                previewNonce.value += 1;
              },
            })}
          </div>
        </Page>
      );
    };
  },
);

export default WidgetsView;
</script>
