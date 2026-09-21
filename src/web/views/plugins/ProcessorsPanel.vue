<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';

import type { AutomationEvent } from '../../../automation/types.ts';
import type { HostMessage, ProcessorStatusEntry } from '../../../shared/messages.ts';
import { processorMetricRows, processorPreviewEvent, processorStatusTone } from '../../components/processors.ts';
import { type Locale } from '../../i18n.ts';
import { pluginCopy } from './plugin-copy.ts';

export type ProcessorTestState = Extract<HostMessage, { type: 'processor-test-result' }>;

/** Host-owned processor diagnostics: status, counters, and sample-event previews. */
type ProcessorsPanelProps = {
  locale: Locale;
  processors: ProcessorStatusEntry[];
  processorTest: ProcessorTestState | null;
  query: string;
  onGetProcessorStatus?: () => void;
  onTestProcessor?: (pluginId: string, processorId: string, event: AutomationEvent) => void;
};

export const ProcessorsPanel = defineVueComponent<ProcessorsPanelProps>(
  ['locale', 'processors', 'processorTest', 'query', 'onGetProcessorStatus', 'onTestProcessor'],
  (props) => {
  return () => {
  const copy = pluginCopy(props.locale);
  const words = props.query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const entries = words.length === 0
    ? props.processors
    : props.processors.filter((entry) => {
        const haystack = `${entry.pluginId} ${entry.processorId} ${entry.status} ${entry.eventTypes.join(' ')}`.toLowerCase();
        return words.every((word) => haystack.includes(word));
      });
  return (
    <>
      <div class="plg-banner">
        <span class="plg-dot is-ok" />
        <span class="plg-banner__label">{copy.processorsTab}</span>
        <span class="plg-banner__note">{copy.processorsLead}</span>
        {props.onGetProcessorStatus && (
          <button type="button" class="plg-btn plg-btn--sm" onClick={() => props.onGetProcessorStatus?.()}>
            {copy.processorsRefresh}
          </button>
        )}
      </div>

      {entries.map((entry) => {
        const test = props.processorTest
          && props.processorTest.pluginId === entry.pluginId
          && props.processorTest.processorId === entry.processorId
          ? props.processorTest
          : null;
        return (
          <div class="plg-plugin" key={`${entry.pluginId}/${entry.processorId}`}>
            <div class="plg-plugin__head">
              <div class="plg-field">
                <div class="plg-plugin__title">
                  <span class={`plg-dot ${processorStatusTone(entry.status)}`} />
                  <span class="plg-plugin__name">{entry.processorId}</span>
                  <span class="plg-pill plg-pill--mono">{entry.pluginId}</span>
                  <span class="plg-pill">{copy.statusLabels[entry.status]}</span>
                </div>
                <div class="plg-table__chips">
                  <span class="plg-group-note">{copy.processorEventsLabel}</span>
                  {entry.eventTypes.length > 0
                    ? entry.eventTypes.map((type) => <span class="plg-pill plg-pill--mono" key={type}>{type}</span>)
                    : <span class="plg-group-note">{copy.processorAllEvents}</span>}
                </div>
                <div class="plg-table__chips">
                  <span class="plg-group-note">{copy.processorMetricsLabel}</span>
                  {processorMetricRows(entry.metrics).map((row) => (
                    <span class="plg-pill" key={row.key}>{copy.metricLabels[row.key] ?? row.key} · {row.value}</span>
                  ))}
                </div>
                <div class="plg-plugin__meta">
                  <span>{copy.processorTimeout} · {entry.timeoutMs} ms</span>
                  <span>·</span>
                  <span>{copy.processorPriority} · {entry.priority}</span>
                </div>
              </div>

              {props.onTestProcessor && (
                <div class="plg-plugin__controls">
                  <button
                    type="button"
                    class="plg-btn plg-btn--sm"
                    onClick={() => props.onTestProcessor?.(entry.pluginId, entry.processorId, processorPreviewEvent(entry))}
                  >
                    {copy.processorTest}
                  </button>
                </div>
              )}
            </div>

            {entry.metrics.lastError && (
              <div class="plg-warn"><span>{entry.metrics.lastError}</span></div>
            )}

            {test && (
              <div class="plg-console">
                <span>{test.ok ? copy.processorTestOk : copy.processorTestFailed} · {copy.processorTestDuration} {test.durationMs} ms</span>
                {test.error && <span>{test.error}</span>}
                <span style="white-space: pre-wrap;">{JSON.stringify(test.result, null, 2)}</span>
              </div>
            )}
          </div>
        );
      })}

      {entries.length === 0 && (
        <div class="plg-empty">
          <span class="plg-empty__desc">
            {words.length > 0 ? copy.searchEmpty(props.query.trim()) : copy.processorsEmpty}
          </span>
        </div>
      )}
    </>
  );
  };
  },
);

export default ProcessorsPanel;
</script>
