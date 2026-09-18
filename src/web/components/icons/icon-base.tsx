import type { VNode, VNodeChild } from 'vue';
import { defineVueFunctional } from '../../vue/component.ts';

export type IconProps = {
  size?: number;
  strokeWidth?: number;
  className?: string;
};

export type IconComponent = (props?: IconProps) => VNode;

/**
 * Shared SVG primitive. Icons inherit `currentColor` so they follow the
 * surrounding text color in both light and dark themes. Built on the shared
 * functional wrapper so JSX children arrive through Vue slots.
 */
export const SvgIcon = defineVueFunctional<IconProps & { filled?: boolean; children?: VNodeChild }>((props) => {
  const { size = 18, strokeWidth = 1.75, className, filled = false, children } = props;
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={filled ? 'currentColor' : 'none'}
      stroke={filled ? 'none' : 'currentColor'}
      stroke-width={strokeWidth}
      stroke-linecap="round"
      stroke-linejoin="round"
      class={className}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
});
