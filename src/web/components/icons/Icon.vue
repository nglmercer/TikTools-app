<script lang="tsx">
import type { VNode } from 'vue';
import { resolveIcon, type IconName, type IconProps } from './icons.tsx';

type IconByNameProps = IconProps & {
  name: IconName | string;
};

/**
 * Data-driven icon: pickers, templates, and canvas rows render an icon by
 * name so no SVG (or emoji) is embedded at the call site. Unknown names fall
 * back to a neutral dot instead of rendering nothing.
 */
export function Icon({ name, size, strokeWidth, className }: IconByNameProps): VNode {
  return resolveIcon(name)({ size, strokeWidth, className });
}

export default Icon;
</script>
