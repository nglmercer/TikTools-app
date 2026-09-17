<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { FieldLabel } from './FieldLabel.vue';
import { FieldMessage } from './FieldMessage.vue';
import { resolveFieldSize, type FieldSize } from './field-logic.ts';

export type FieldShellProps = {
  /** Control id: label `for` target and message-id base. */
  id: string;
  label?: string;
  /** Tooltip-only explanation (ⓘ). Never rendered as a paragraph. */
  hint?: string;
  hintPosition?: TooltipPosition;
  /** Visible help under the control. Hidden while an error shows. */
  description?: string;
  error?: string;
  required?: boolean;
  disabled?: boolean;
  invalid?: boolean;
  size?: FieldSize;
  className?: string;
  /** The control row (usually InputGroup). Rendered inside the one box. */
  children: VNodeChild;
};

/**
 * Sole owner of label/tooltip/message/disabled/focus/border/background/
 * radius/sizing. Structure is label-above-input, one bordered box, one
 * message line. Floating labels are intentionally not supported: every field
 * on a page shares this single label system.
 */
export const FieldShell = defineVueComponent<FieldShellProps>(
  ['id', 'label', 'hint', 'hintPosition', 'description', 'error', 'required', 'disabled', 'invalid', 'size', 'className', 'children'],
  (props) => {
    return () => {
      const size = resolveFieldSize(props.size);
      const invalid = props.invalid ?? Boolean(props.error);
      const className = `field field--${size}${invalid ? ' is-invalid' : ''}${props.disabled ? ' is-disabled' : ''}${props.className ? ` ${props.className}` : ''}`;
      return (
        <div class={className}>
          {props.label ? (
            <FieldLabel
              label={props.label}
              htmlFor={props.id}
              required={props.required}
              hint={props.hint}
              hintPosition={props.hintPosition}
            />
          ) : null}
          <div class="field__box">{props.children}</div>
          <FieldMessage id={props.id} description={props.description} error={props.error} />
        </div>
      );
    };
  },
);

export default FieldShell;
</script>
