<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import { Icon } from '../../components/icons/Icon.vue';
import { Button } from '../../components/ui/Button.vue';
import { i18nText, type Locale } from '../../i18n.ts';
import type { RuleTemplate } from './rule-templates.ts';

export type StagedTemplateListProps = {
  locale: Locale;
  /** Pre-translated headline, e.g. the profile name or template count. */
  headline: string;
  templates: RuleTemplate[];
  source: string;
  stagedFrom: string;
  clearLabel: string;
  /** Optional pre-translated note rendered under the list. */
  hint?: string;
  onClear: () => void;
};

/** Shared staged-import preview: what would be imported, before it is. */
export const StagedTemplateList = defineVueComponent<StagedTemplateListProps>(
  ['locale', 'headline', 'templates', 'source', 'stagedFrom', 'clearLabel', 'hint', 'onClear'],
  (props) => () => (
    <div class="rule-import-staged" role="status">
      <div class="rule-import-staged__head">
        <strong>{props.headline}</strong>
        <span class="rule-import-staged__source">{props.stagedFrom}</span>
        <Button variant="ghost" size="sm" onClick={() => props.onClear()}>
          {props.clearLabel}
        </Button>
      </div>
      <ul>
        {props.templates.map((template) => (
          <li key={template.id}>
            <Icon name={template.icon} size={14} />
            <span class="rule-import-staged__name">{i18nText(props.locale, template.title)}</span>
            <small>{template.event.trigger}</small>
          </li>
        ))}
      </ul>
      {props.hint ? <p class="rule-import-hint">{props.hint}</p> : null}
    </div>
  ),
);

export default StagedTemplateList;
</script>

<style scoped>
.rule-import-staged {
  border: 1px solid var(--tt-border, #34343f);
  border-radius: 12px;
  padding: 12px;
  background: var(--tt-bg-soft, #1c1c26);
}

.rule-import-staged__head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.rule-import-staged__source {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 12px;
}

.rule-import-staged ul {
  margin: 8px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  max-height: 220px;
  overflow-y: auto;
}

.rule-import-staged li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 2px;
  font-size: 13px;
  border-top: 1px solid var(--tt-border, #34343f);
}

.rule-import-staged li:first-child {
  border-top: 0;
}

.rule-import-staged__name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-import-staged li small {
  margin-left: auto;
  flex: none;
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--tt-bg, #17171f);
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 11px;
}

.rule-import-hint {
  margin: 8px 0 0;
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 13px;
}
</style>
