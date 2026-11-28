import { shallowRef } from 'vue';
import type { GuidePause } from './types.ts';

/** The guide the app interrupted (route change, dialog) and offers to resume; `undefined` when nothing is paused. */
export const pausedGuide = shallowRef<GuidePause | undefined>();
