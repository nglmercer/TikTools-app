<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueFunctional } from '../../vue/component.ts';
import { Checkbox } from './Checkbox.vue';
import { InfoTip } from './InfoTip.vue';

type SettingRowProps = {
  children: VNodeChild;
  /** Row label; keep it short, explanations go behind `hint`. */
  label: string;
  /** Longer explanation hidden behind an info icon. Omit for warnings. */
  hint?: string;
  /** When defined, an enable toggle renders before the control. */
  enabled?: boolean;
  onEnabledChange?: (value: boolean) => void;
  disabled?: boolean;
};

/**
 * Standard compact settings row: `Label ⓘ … [toggle] [control]`. No
 * paragraph underneath; warnings and must-see text stay inline instead.
 */
export const SettingRow = defineVueFunctional<SettingRowProps>((props) => {
  const { children, label, hint, enabled, onEnabledChange, disabled } = props;
  return (
    <div class="ui-setting-row">
      <span class="ui-setting-row__label">
        <span>{label}</span>
        {hint ? <InfoTip text={hint} /> : null}
      </span>
      <span class="ui-setting-row__controls">
        {enabled !== undefined ? (
          <Checkbox
            checked={enabled}
            onCheckedChange={(value) => onEnabledChange?.(value)}
            disabled={disabled}
            ariaLabel={label}
          />
        ) : null}
        {children}
      </span>
    </div>
  );
});

export default SettingRow;
</script>
