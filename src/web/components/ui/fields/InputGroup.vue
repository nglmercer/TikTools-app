<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';

export type InputGroupProps = {
  /** Decorative icon before the control (must be self-hidden, e.g. `Icon`). */
  leading?: VNodeChild;
  /** Decorative icon or interactive content after the control. */
  trailing?: VNodeChild;
  /** Semantic text pinned before the input (`@`, `$`). Not an icon. */
  prefix?: string;
  /** Semantic text pinned after the input (`ms`, `%`). Not an icon. */
  suffix?: string;
  focused?: boolean;
  invalid?: boolean;
  disabled?: boolean;
  children: VNodeChild;
};

/**
 * Inner control row: leading / prefix / control / suffix / trailing. It owns
 * layout only — never a second border or background. The surrounding
 * FieldShell box is the one visible border.
 */
export const InputGroup = defineVueComponent<InputGroupProps>(
  ['leading', 'trailing', 'prefix', 'suffix', 'focused', 'invalid', 'disabled', 'children'],
  (props) => {
    return () => (
      <div
        class={`field-group${props.focused ? ' is-focused' : ''}${props.invalid ? ' is-invalid' : ''}${props.disabled ? ' is-disabled' : ''}`}
      >
        {props.leading ? <span class="field-group__leading">{props.leading}</span> : null}
        {props.prefix ? <span class="field-group__prefix">{props.prefix}</span> : null}
        {props.children}
        {props.suffix ? <span class="field-group__suffix">{props.suffix}</span> : null}
        {props.trailing ? <span class="field-group__trailing">{props.trailing}</span> : null}
      </div>
    );
  },
);

export default InputGroup;
</script>
