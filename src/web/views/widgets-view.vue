<script lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { IconCheck, IconCopy, IconFollow, IconGift, IconRefresh } from '../components/icons.vue';
import { Button } from '../components/ui/Button.vue';
import { Card } from '../components/ui/Card.vue';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { TextInput } from '../components/ui/TextInput.vue';
import {
  buildRedactedObsUrl,
  buildWidgetPreviewUrl,
  GATEWAY_DEFAULT_PORT,
  type WidgetKind,
  type WidgetsCopyFeedback,
  type WidgetsHostState,
  type WidgetsStatus,
} from '../features/widgets.ts';
import { t, type Locale } from '../i18n.ts';
import { defineVueComponent } from '../vue/component.ts';

type WidgetsViewProps = {
  locale: Locale;
  status: WidgetsStatus | null;
  statusError: string | null;
  refreshing: boolean;
  onRefresh: () => void;
  onCopy: (widget: WidgetKind, onDone: (feedback: WidgetsCopyFeedback) => void) => void;
};

/** Bounded re-probe while the gateway reports `starting`. */
const STARTING_RETRIES = 5;
const STARTING_RETRY_MS = 2000;

function renderWidgetCard(args: {
  locale: Locale;
  widget: WidgetKind;
  title: string;
  description: string;
  icon: typeof IconGift;
  obsUrl: string;
  copyReady: boolean;
  previewUrl: string;
  previewKey: number;
  copied: boolean;
  copyError: string | null;
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
          disabled={!args.copyReady}
        >
          {args.copied ? t(locale, 'widgetsCopied') : t(locale, 'widgetsCopyUrl')}
        </Button>
      </div>
      {args.copyError ? <p class="widgets-hint">{args.copyError}</p> : null}
      <p class="widgets-token-note">{t(locale, 'widgetsRedactedNote')}</p>
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
 * Widgets: OBS Browser Source URLs for the Follow and Gift alerts. State
 * and copying are host-driven (`widgets.status`, `widgets.copyObsUrl`):
 * the page only renders the host state and a redacted URL placeholder,
 * never the real credential. Previews run the widget in local demo mode,
 * which needs no gateway connection.
 */
export const WidgetsView = defineVueComponent<WidgetsViewProps>(
  ['locale', 'status', 'statusError', 'refreshing', 'onRefresh', 'onCopy'],
  (props) => {
    const previewNonce = ref(0);
    const copiedWidget = ref<WidgetKind | null>(null);
    const copyError = ref<{ widget: WidgetKind; message: string } | null>(null);
    const startingAttempts = ref(0);
    let copiedTimer: ReturnType<typeof setTimeout> | undefined;
    let retryTimer: ReturnType<typeof setTimeout> | undefined;

    const port = computed(() => props.status?.port ?? GATEWAY_DEFAULT_PORT);
    const state = computed<WidgetsHostState | null>(() => props.status?.state ?? null);

    const clearRetry = (): void => {
      if (retryTimer) {
        clearTimeout(retryTimer);
        retryTimer = undefined;
      }
    };

    onMounted(() => {
      props.onRefresh();
    });

    onUnmounted(() => {
      clearRetry();
      if (copiedTimer) clearTimeout(copiedTimer);
    });

    watch(
      () => props.status,
      (next) => {
        clearRetry();
        if (next?.state === 'starting' && startingAttempts.value < STARTING_RETRIES) {
          startingAttempts.value += 1;
          retryTimer = setTimeout(() => {
            props.onRefresh();
          }, STARTING_RETRY_MS);
        }
      },
      { immediate: true },
    );

    const copyWidget = (widget: WidgetKind): void => {
      copyError.value = null;
      props.onCopy(widget, (feedback) => {
        if (copiedTimer) clearTimeout(copiedTimer);
        if (feedback.ok) {
          copiedWidget.value = widget;
          copiedTimer = setTimeout(() => {
            copiedWidget.value = null;
          }, 2000);
        } else {
          copiedWidget.value = null;
          copyError.value = { widget, message: feedback.message };
        }
      });
    };

    const manualRefresh = (): void => {
      startingAttempts.value = 0;
      props.onRefresh();
    };

    const statusText = computed(() => {
      switch (state.value) {
        case 'missing':
          return t(props.locale, 'widgetsStateMissing');
        case 'disabled':
          return t(props.locale, 'widgetsStateDisabled');
        case 'stopped':
          return t(props.locale, 'widgetsStateStopped');
        case 'starting':
          return t(props.locale, 'widgetsStateStarting');
        case 'credential-unavailable':
          return t(props.locale, 'widgetsStateCredentialUnavailable');
        case 'assets-missing':
          return t(props.locale, 'widgetsStateAssetsMissing');
        case 'unreachable':
          return t(props.locale, 'widgetsStateUnreachable');
        case 'ready':
          return t(props.locale, 'widgetsStateReady', { port: port.value });
        default:
          return t(props.locale, 'widgetsStateUnknown');
      }
    });

    return () => {
      const locale = props.locale;
      const current = state.value;
      const followUrl = buildRedactedObsUrl(port.value, 'follow');
      const giftUrl = buildRedactedObsUrl(port.value, 'gift');
      const followPreview = buildWidgetPreviewUrl(port.value, 'follow');
      const giftPreview = buildWidgetPreviewUrl(port.value, 'gift');
      const healthy = current === 'ready';
      // Copy needs a stored credential; the host reports that through
      // every state except the ones that prove it missing.
      const copyReady =
        current !== null &&
        current !== 'missing' &&
        current !== 'credential-unavailable' &&
        !props.refreshing;
      const hint = props.statusError ?? props.status?.error ?? null;
      const copyErrorFor = (widget: WidgetKind): string | null =>
        copyError.value?.widget === widget ? copyError.value.message : null;
      return (
        <Page width="wide">
          <PageHeader
            title={t(locale, 'tabWidgets')}
            subtitle={t(locale, 'widgetsSubtitle')}
            icon={<IconGift />}
            action={
              <Button
                variant="soft"
                size="md"
                icon={<IconRefresh size={14} />}
                onClick={manualRefresh}
              >
                {t(locale, 'widgetsRefresh')}
              </Button>
            }
          />
          <Card title={t(locale, 'widgetsGateway')} icon={<IconGift />}>
            <div class="widgets-status-row">
              <span class={`ui-badge ${healthy ? 'ui-badge--cyan' : ''}`}>{statusText.value}</span>
              {props.refreshing ? (
                <span class="widgets-health">{t(locale, 'widgetsStateChecking')}</span>
              ) : null}
            </div>
            {hint ? <p class="widgets-hint">{hint}</p> : null}
          </Card>
          <div class="ui-cols-2">
            {renderWidgetCard({
              locale,
              widget: 'follow',
              title: t(locale, 'widgetsFollowTitle'),
              description: t(locale, 'widgetsFollowDescription'),
              icon: IconFollow,
              obsUrl: followUrl,
              copyReady,
              previewUrl: followPreview,
              previewKey: previewNonce.value,
              copied: copiedWidget.value === 'follow',
              copyError: copyErrorFor('follow'),
              onCopy: () => copyWidget('follow'),
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
              copyReady,
              previewUrl: giftPreview,
              previewKey: previewNonce.value,
              copied: copiedWidget.value === 'gift',
              copyError: copyErrorFor('gift'),
              onCopy: () => copyWidget('gift'),
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
