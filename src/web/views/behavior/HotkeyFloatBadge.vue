<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';

import type { PluginStatus } from '../../../automation/behavior/types.ts';
import type { HotkeyStatusData } from '../../../shared/messages.ts';
import type { LastHotkeyEvent } from '../../features/automation.ts';
import { Icon } from '../../components/icons/index.ts';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import { formatHotkeyChord, hotkeyListenerState, hotkeyNeedsSeatAccess, summarizeHotkeyStatus } from '../../components/ui/hotkey-status.ts';
import { t, type Locale } from '../../i18n.ts';
import { relativeTime } from './helpers.vue';

type HotkeyFloatBadgeProps = {
  locale: Locale;
  plugins: PluginStatus[];
  hotkeyStatus?: HotkeyStatusData | null;
  lastHotkeyEvent?: LastHotkeyEvent | null;
  accessPending?: boolean;
  onRequestAccess?: () => void;
};

/**
 * Listener status while the hotkeys plugin is installed, collapsed into a
 * floating badge; the tooltip carries the full headline, diagnostics, and
 * last event.
 */
export const HotkeyFloatBadge = defineVueComponent<HotkeyFloatBadgeProps>(
  ['locale', 'plugins', 'hotkeyStatus', 'lastHotkeyEvent', 'accessPending', 'onRequestAccess'],
  (props) => {
  return () => {
  const locale = props.locale;
  const hotkeySummary = props.hotkeyStatus ? summarizeHotkeyStatus(props.hotkeyStatus) : null;
  const hotkeyPanel = (() => {
    const plugin = props.plugins.find((entry) => entry.descriptor.id === 'hotkeys');
    if (!plugin?.installed) return null;
    if (!plugin.enabled) {
      return {
        tone: 'idle' as const,
        headline: t(locale, 'hotkeyStateDisabled'),
        lines: [] as string[],
        lastEvent: null as string | null,
      };
    }
    const state = hotkeyListenerState(props.hotkeyStatus);
    const headline = state === 'active'
      ? `${t(locale, 'hotkeyStateActive')} · ${(hotkeySummary?.lines ?? []).find((line) => line.includes('via')) ?? hotkeySummary?.headline ?? ''}`
      : state === 'starting' || state === 'unknown'
        ? t(locale, 'hotkeyStateStarting')
        : state === 'permission'
          ? t(locale, 'hotkeyStatePermission')
          : state === 'failed'
            ? t(locale, 'hotkeyStateFailed')
            : t(locale, 'hotkeyStateUnsupported');
    const tone = state === 'active' ? 'ok' as const : (state === 'permission' || state === 'failed') ? 'err' as const : 'idle' as const;
    const summary = hotkeySummary;
    const lines = summary && (summary.needsAttention || state !== 'active')
      ? summary.lines.filter((line) => line !== summary.headline)
      : [];
    const last = props.lastHotkeyEvent;
    const lastEvent = last
      ? `${t(locale, 'hotkeyLastEvent')}: ${formatHotkeyChord(last.key, last.modifiers)} · ${relativeTime(last.at, locale)}`
      : state === 'active' || state === 'starting' || state === 'unknown'
        ? t(locale, 'hotkeyStateNoEvents')
        : null;
    return { tone, headline, lines, lastEvent };
  })();

  if (!hotkeyPanel) return null;
  const hotkeyTip = [t(locale, 'hotkeyStatusTitle'), hotkeyPanel.headline, ...hotkeyPanel.lines, hotkeyPanel.lastEvent]
    .filter((part): part is string => typeof part === 'string' && part.length > 0)
    .join(' — ');

  const showGrantAccess = hotkeyPanel.tone === 'err'
    && hotkeyNeedsSeatAccess(props.hotkeyStatus)
    && typeof props.onRequestAccess === 'function';

  return (
    <span class="plg-hotkey-float">
      {showGrantAccess && (
        <button
          type="button"
          class="plg-btn plg-btn--sm"
          style="margin-right: 8px;"
          disabled={props.accessPending === true}
          onClick={() => props.onRequestAccess?.()}
        >
          {t(locale, 'hotkeyGrantAccess')}
        </button>
      )}
      <Tooltip text={hotkeyTip} position="left">
        <span class="plg-hotkey-float__body" role="img" aria-label={hotkeyTip}>
          <Icon name="keyboard" size={15} />
          <span
            class={`plg-dot${hotkeyPanel.tone === 'err' ? ' is-err' : hotkeyPanel.tone === 'ok' ? ' is-ok' : ''}`}
            aria-hidden="true"
          />
        </span>
      </Tooltip>
    </span>
  );
  };
  },
);

export default HotkeyFloatBadge;
</script>
