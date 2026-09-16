<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueFunctional } from '../../vue/component.ts';
import { Tooltip } from './Tooltip.vue';

type ButtonProps = {
  children?: VNodeChild;
  variant?: 'primary' | 'soft' | 'ghost' | 'danger' | 'cyan';
  size?: 'sm' | 'md' | 'lg';
  block?: boolean;
  loading?: boolean;
  disabled?: boolean;
  icon?: VNodeChild;
  iconOnly?: boolean;
  type?: 'button' | 'submit' | 'reset';
  onClick?: () => void;
  tooltip?: string;
};

export const Button = defineVueFunctional<ButtonProps>((props) => {
  const {
    children,
    variant = 'soft',
    size = 'md',
    block,
    loading,
    disabled,
    icon,
    iconOnly,
    type = 'button',
    onClick,
    tooltip,
  } = props;
  const node = (
    <button
      type={type}
      disabled={disabled || loading}
      aria-label={iconOnly ? tooltip : undefined}
      onClick={onClick}
      class={`ui-btn ui-btn--${variant} ui-btn--${size} ${block ? 'is-block' : ''} ${iconOnly ? 'is-icon-only' : ''} ${loading ? 'is-loading' : ''}`}
    >
      {icon ? <span class="ui-btn__icon">{icon}</span> : null}
      {!iconOnly ? <span class="ui-btn__label">{loading ? '…' : children}</span> : null}
    </button>
  );
  // Portal tooltip: escapes stacking contexts and still shows on disabled
  // buttons, whose own mouse events never fire.
  return tooltip ? <Tooltip text={tooltip} position="top">{node}</Tooltip> : node;
});

export default Button;
</script>
