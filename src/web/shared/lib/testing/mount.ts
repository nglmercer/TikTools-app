import { mount, type ComponentMountingOptions } from '@vue/test-utils';
import type { App, Component } from 'vue';
import { MotionPlugin } from '@vueuse/motion';
import { i18n } from '@/shared/i18n/i18n.ts';

const feedMotion = {
  install(app: App): void {
    app.use(MotionPlugin, { excludePresets: true });
  },
};

export function mountWithPlugins<C extends Component>(component: C, options: ComponentMountingOptions<C> = {}) {
  return mount(component, {
    ...options,
    global: { ...options.global, plugins: [i18n, feedMotion, ...(options.global?.plugins ?? [])] },
  } as ComponentMountingOptions<C>);
}
