<script lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import {
  IconCheck,
  IconCopy,
  IconEdit,
  IconRefresh,
} from '../components/icons.vue';
import widgetsOverlayGraphic from '../assets/widgets-overlay.svg';
import { Button } from '../components/ui/Button.vue';
import { Card } from '../components/ui/Card.vue';
import WidgetHost from '../../widgets/sdk/WidgetHost.vue';
import { widgetTemplates } from '../../widgets/sdk/templates.ts';
import { Page, PageHeader } from '../components/ui/Page.vue';
import { TextInput } from '../components/ui/TextInput.vue';
import {
  buildRedactedObsUrl,
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

type WidgetDefinition = {
  kind: WidgetKind;
  tabLabel: string;
  title: string;
  description: string;
};

/** Bounded re-probe while the gateway reports `starting`. */
const STARTING_RETRIES = 5;
const STARTING_RETRY_MS = 2000;

function renderWidgetTab(args: {
  definition: WidgetDefinition;
  selected: boolean;
  onSelect: (widget: WidgetKind) => void;
}) {
  const { definition } = args;
  return (
    <button
      id={`widgets-tab-${definition.kind}`}
      type="button"
      role="tab"
      aria-selected={args.selected}
      aria-controls="widgets-detail-panel"
      class={`widgets-tab ${args.selected ? 'is-active' : ''}`}
      onClick={() => args.onSelect(definition.kind)}
    >
      <span>{definition.tabLabel}</span>
    </button>
  );
}

function renderWidgetPanel(args: {
  locale: Locale;
  definition: WidgetDefinition;
  obsUrl: string;
  previewKey: number;
  copyReady: boolean;
  copied: boolean;
  copyError: string | null;
  healthy: boolean;
  statusText: string;
  refreshing: boolean;
  onCopy: () => void;
  onRefresh: () => void;
  onReplay: () => void;
}) {
  const { locale, definition } = args;
  return (
    <section
      id="widgets-detail-panel"
      class="widgets-detail-panel"
      role="tabpanel"
      aria-labelledby={`widgets-tab-${definition.kind}`}
    >
      <Card
        className="widgets-detail-card"
        title={definition.title}
        action={
          <div class="widgets-panel-status">
            <span class={`widgets-status-badge ${args.healthy ? 'is-active' : 'is-muted'}`}>
              <span class="widgets-status-badge__dot" aria-hidden="true" />
              {args.healthy ? t(locale, 'widgetsActive') : args.statusText}
            </span>
            <Button
              variant="ghost"
              size="sm"
              icon={<IconRefresh size={14} />}
              iconOnly
              tooltip={t(locale, 'widgetsRefresh')}
              loading={args.refreshing}
              onClick={args.onRefresh}
            />
          </div>
        }
      >
        <p class="widgets-description">{definition.description}</p>

        <div class="widgets-field-label">{t(locale, 'widgetsObsUrl')}</div>
        <div class="widgets-url-row">
          <TextInput
            value={args.obsUrl}
            onValueChange={() => {}}
            readonly
            size="sm"
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
            {args.copied ? t(locale, 'widgetsCopied') : t(locale, 'widgetsCopyUrlShort')}
          </Button>
        </div>
        {args.copyError ? <p class="widgets-hint">{args.copyError}</p> : null}
        <p class="widgets-token-note">{t(locale, 'widgetsRedactedNote')}</p>

        <div class="widgets-field-label">{t(locale, 'widgetsPreview')}</div>
        <div class="widgets-preview-frame" aria-label={t(locale, 'widgetsPreview')}>
          <WidgetHost
            template={widgetTemplates[definition.kind]}
            mode="preview"
            replayKey={args.previewKey}
          />
        </div>

        <div class="widgets-preview-actions">
          <button
            class="widgets-edit-design"
            type="button"
            disabled
            title={t(locale, 'widgetsEditDesignHint')}
            aria-label={`${t(locale, 'widgetsEditDesign')}: ${t(locale, 'widgetsEditDesignHint')}`}
          >
            <IconEdit size={17} />
            <span>{t(locale, 'widgetsEditDesign')}</span>
            <span class="widgets-edit-design__arrow" aria-hidden="true">›</span>
          </button>
          <Button
            variant="ghost"
            size="lg"
            icon={<IconRefresh size={17} />}
            block
            onClick={args.onReplay}
          >
            {t(locale, 'widgetsReplay')}
          </Button>
        </div>
      </Card>
    </section>
  );
}

/**
 * Widgets: one focused detail panel at a time. State and copying are
 * host-driven (`widgets.status`, `widgets.copyObsUrl`): the page only renders
 * the host state and a redacted URL placeholder, never the real credential.
 * Previews run the widget in local demo mode, which needs no gateway
 * connection.
 */
export const WidgetsView = defineVueComponent<WidgetsViewProps>(
  ['locale', 'status', 'statusError', 'refreshing', 'onRefresh', 'onCopy'],
  (props) => {
    const previewNonce = ref(0);
    const selectedKind = ref<WidgetKind>('follow');
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

    const selectWidget = (widget: WidgetKind): void => {
      selectedKind.value = widget;
      copyError.value = null;
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
      const healthy = current === 'ready';
      // Copy needs a stored credential; the host reports that through every
      // state except the ones that prove it missing.
      const copyReady =
        current !== null &&
        current !== 'missing' &&
        current !== 'credential-unavailable' &&
        !props.refreshing;
      const hint = props.statusError ?? props.status?.error ?? null;
      const definitions: WidgetDefinition[] = [
        {
          kind: 'follow',
          tabLabel: t(locale, 'widgetsFollowTab'),
          title: t(locale, 'widgetsFollowTitle'),
          description: t(locale, 'widgetsFollowDescription'),
        },
        {
          kind: 'gift',
          tabLabel: t(locale, 'widgetsGiftTab'),
          title: t(locale, 'widgetsGiftTitle'),
          description: t(locale, 'widgetsGiftDescription'),
        },
        {
          kind: 'chat',
          tabLabel: t(locale, 'widgetsChatTab'),
          title: t(locale, 'widgetsChatTitle'),
          description: t(locale, 'widgetsChatDescription'),
        },
        {
          kind: 'share',
          tabLabel: t(locale, 'widgetsShareTab'),
          title: t(locale, 'widgetsShareTitle'),
          description: t(locale, 'widgetsShareDescription'),
        },
        {
          kind: 'subscribe',
          tabLabel: t(locale, 'widgetsSubscribeTab'),
          title: t(locale, 'widgetsSubscribeTitle'),
          description: t(locale, 'widgetsSubscribeDescription'),
        },
      ];
      const selected = definitions.find((definition) => definition.kind === selectedKind.value) ?? definitions[0];
      if (!selected) return null;
      const copyErrorFor = (widget: WidgetKind): string | null =>
        copyError.value?.widget === widget ? copyError.value.message : null;

      return (
        <Page width="wide" className="widgets-page">
          <PageHeader
            title={t(locale, 'tabWidgets')}
            subtitle={t(locale, 'widgetsSubtitle')}
            icon={<img class="widgets-overlay-mark" src={widgetsOverlayGraphic} alt="" aria-hidden="true" />}
          />

          <nav class="widgets-tabs" role="tablist" aria-label={t(locale, 'tabWidgets')}>
            {definitions.map((definition) =>
              renderWidgetTab({
                definition,
                selected: selected.kind === definition.kind,
                onSelect: selectWidget,
              }),
            )}
          </nav>

          {hint ? <p class="widgets-gateway-hint">{hint}</p> : null}

          {renderWidgetPanel({
            locale,
            definition: selected,
            obsUrl: buildRedactedObsUrl(port.value, selected.kind),
            previewKey: previewNonce.value,
            copyReady,
            copied: copiedWidget.value === selected.kind,
            copyError: copyErrorFor(selected.kind),
            healthy,
            statusText: statusText.value,
            refreshing: props.refreshing,
            onCopy: () => copyWidget(selected.kind),
            onRefresh: manualRefresh,
            onReplay: () => {
              previewNonce.value += 1;
            },
          })}
        </Page>
      );
    };
  },
);

export default WidgetsView;
</script>
