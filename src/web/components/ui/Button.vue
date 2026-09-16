<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueFunctional } from '../../vue/component.ts';

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
  return (
    <button
      type={type}
      disabled={disabled || loading}
      data-tooltip={tooltip}
      data-tooltip-pos="top"
      aria-label={iconOnly ? tooltip : undefined}
      onClick={onClick}
      class={`ui-btn ui-btn--${variant} ui-btn--${size} ${block ? 'is-block' : ''} ${iconOnly ? 'is-icon-only' : ''} ${loading ? 'is-loading' : ''}`}
    >
      {icon ? <span class="ui-btn__icon">{icon}</span> : null}
      {!iconOnly ? <span class="ui-btn__label">{loading ? '…' : children}</span> : null}
    </button>
  );
});

export default Button;
</script>
