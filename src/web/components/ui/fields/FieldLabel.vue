<script lang="tsx">
import { InfoTip } from '../InfoTip.vue';
import type { TooltipPosition } from '../tooltip-logic.ts';

export type FieldLabelProps = {
  label: string;
  htmlFor?: string;
  required?: boolean;
  /** Tooltip-only explanation (ⓘ). Never rendered as a paragraph. */
  hint?: string;
  hintPosition?: TooltipPosition;
};

/**
 * Canonical label row for the shared field system: label-above-input with an
 * optional ⓘ tooltip. (The plugin-scoped `ui/FieldLabel.vue` keeps its own
 * `plg-*` styling for compatibility; new fields use this one.)
 */
export function FieldLabel({ label, htmlFor, required, hint, hintPosition = 'right' }: FieldLabelProps) {
  return (
    <div class="field-label-row">
      <label class="field-label" for={htmlFor}>
        {label}
        {required ? <span class="field-label__req" aria-hidden="true">*</span> : null}
      </label>
      {hint ? <InfoTip text={hint} position={hintPosition} /> : null}
    </div>
  );
}

export default FieldLabel;
</script>
