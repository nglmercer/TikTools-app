<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent, defineVueFunctional } from '../../vue/component.ts';
import type { BehaviorRun } from '../../../automation/behavior/types.ts';
import { t, type Locale } from '../../i18n.ts';
import { InfoTip } from './InfoTip.vue';

/** Collapsible group for advanced fields, with a count badge + tooltip. */
export const AdvancedSection = defineVueFunctional<{
  title: string;
  hint?: string;
  count?: number;
  children?: VNodeChild;
  defaultOpen?: boolean;
}>((props) => {
  const { title, hint, count, children, defaultOpen = false } = props;
  return (
    <details class="plg-details" open={defaultOpen || undefined}>
      <summary>
        <span>{title}</span>
        {typeof count === 'number' && count > 0 ? <span class="plg-details__count">{count}</span> : null}
        {hint ? (
          <span class="plg-details__tip" onClick={(event) => event.preventDefault()}>
            <InfoTip text={hint} position="right" />
          </span>
        ) : null}
      </summary>
      <div class="plg-details__body">{children}</div>
    </details>
  );
});

/** Right-side cards: derived network allowlist and engine capabilities. */
export function PermissionCards({
  locale,
  network,
  capabilities,
  noneLabel,
}: {
  locale: Locale;
  network: string[];
  capabilities: string[];
  noneLabel: string;
}) {
  return (
    <section aria-label={t(locale, 'behavior.editor.permsTitle')}>
      <div class="act-side-title">{t(locale, 'behavior.editor.permsTitle')}</div>
      <div class="act-cards">
        <div class="act-card">
          <span class="act-card__label">{t(locale, 'behavior.editor.networkCard')}</span>
          {network.length === 0 && <span class="act-card__empty">{noneLabel}</span>}
          {network.map((host) => (
            <span class="act-card__row" key={host}>
              <i class="act-dot is-net" aria-hidden="true" />
              <code>{host}</code>
            </span>
          ))}
        </div>
        <div class="act-card">
          <span class="act-card__label">{t(locale, 'behavior.editor.capsCard')}</span>
          {capabilities.length === 0 && <span class="act-card__empty">{noneLabel}</span>}
          {capabilities.map((capability) => (
            <span class="act-card__row" key={capability}>
              <i class="act-dot is-cap" aria-hidden="true" />
              <code>{capability}</code>
            </span>
          ))}
        </div>
      </div>
    </section>
  );
}

function parseRunStatus(run: BehaviorRun | undefined): { code?: string; ok: boolean; text: string } {
  if (!run) return { ok: true, text: '' };
  const code = /(\d{3})/.exec(`${run.summary} ${run.logs.join(' ')}`)?.[1];
  if (run.status === 'error') return { code, ok: false, text: run.error ?? run.summary };
  return { code, ok: true, text: code ? `${code} OK` : run.summary };
}

/** Right-side test console: run button, status pill and response viewer. */
export const TestConsole = defineVueComponent<{
  locale: Locale;
  run?: BehaviorRun;
  /** Configured request headers shown under the headers tab (fetch only). */
  headers?: Record<string, string>;
  onRun: () => void;
  emptyLabel: string;
}>(['locale', 'run', 'headers', 'onRun', 'emptyLabel'], (props) => {
  const tab = ref<'response' | 'headers'>('response');

  return () => {
    const { locale, run, headers, onRun, emptyLabel } = props;
    const status = parseRunStatus(run);
    const headerEntries = Object.entries(headers ?? {});
    const showTabs = headerEntries.length > 0;
    return (
    <section aria-label={t(locale, 'behavior.editor.consoleTitle')}>
      <div class="act-console-head">
        <span class="act-side-title act-side-title--inline">
          <i class={`act-dot ${run ? (status.ok ? 'is-net' : 'is-err') : ''}`} aria-hidden="true" />
          {t(locale, 'behavior.editor.consoleTitle')}
        </span>
        <button type="button" class="act-runbtn" onClick={onRun}>
          <span class="act-runbtn__icon" aria-hidden="true">▶</span>
          {t(locale, 'behavior.editor.runTest')}
        </button>
      </div>

      {run && (
        <div class="act-status">
          <span class={`act-pill ${status.ok ? 'is-ok' : 'is-err'}`}>{status.text}</span>
          <span class="act-ms">{run.durationMs} ms{run.eventSource ? ` · ${run.eventSource === 'live' ? t(locale, 'behavior.copy.testLive') : t(locale, 'behavior.copy.testSample')}` : ''}</span>
        </div>
      )}

      {showTabs && (
        <div class="act-tabs act-tabs--console" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab.value === 'response'}
            class={`act-tab${tab.value === 'response' ? ' is-active' : ''}`}
            onClick={() => (tab.value = 'response')}
          >
            {t(locale, 'behavior.editor.responseTab')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab.value === 'headers'}
            class={`act-tab${tab.value === 'headers' ? ' is-active' : ''}`}
            onClick={() => (tab.value = 'headers')}
          >
            {t(locale, 'behavior.editor.respHeadersTab', { count: headerEntries.length })}
          </button>
        </div>
      )}

      <div class="act-code" role="log" aria-label={t(locale, 'behavior.editor.consoleTitle')}>
        {(!showTabs || tab.value === 'response') && (
          run && run.logs.length > 0
            ? run.logs.map((line, index) => <div key={`${index}-${line}`}>{line}</div>)
            : <span class="act-code__empty">{emptyLabel}</span>
        )}
        {showTabs && tab.value === 'headers' && headerEntries.map(([key, value]) => (
          <div key={key} class="act-code__kv">
            <span class="act-code__k">{key}:</span> <span class="act-code__v">{value}</span>
          </div>
        ))}
      </div>
    </section>
    );
  };
});

/** Key/value row with per-button tooltips (rename, edit, remove). */
export const KeyValueRowTip = defineVueFunctional<{ children?: VNodeChild }>((props) => <div class="plg-kv-row">{props.children}</div>);

export default PermissionCards;
</script>
