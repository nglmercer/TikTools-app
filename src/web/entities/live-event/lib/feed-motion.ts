import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue';
import { useReducedMotion, type MotionVariants } from '@vueuse/motion';
import type { DisplayEvent } from '@/types.ts';

export type FeedMotion = MotionVariants<string>;

const BURST_WINDOW_MS = 200;
const BURST_LIMIT = 2;

/** Entry motion for feed rows. Resolved once per list so each card does not
 *  register its own media-query listener. Transform animations are avoided:
 *  in a scrolling, windowed list they fight the scroll pin and look like jumps.
 *  Under a burst, rows render without animation altogether. */
export function useFeedMotion(events: Ref<readonly DisplayEvent[]> | (() => readonly DisplayEvent[])) {
  const reduced = useReducedMotion();
  const animate = ref(true);
  let burstTimer: ReturnType<typeof setTimeout> | null = null;

  watch(events, (list) => {
    const recent = list.filter((event) => Date.now() - event.receivedAt < BURST_WINDOW_MS).length;
    if (recent <= BURST_LIMIT) return;
    animate.value = false;
    if (burstTimer !== null) clearTimeout(burstTimer);
    burstTimer = setTimeout(() => {
      animate.value = true;
      burstTimer = null;
    }, BURST_WINDOW_MS);
  });

  onBeforeUnmount(() => {
    if (burstTimer !== null) clearTimeout(burstTimer);
  });

  const still: FeedMotion = {
    initial: { opacity: 1 },
    enter: { opacity: 1 },
  };

  const fade: FeedMotion = {
    initial: { opacity: 0 },
    enter: { opacity: 1, transition: { duration: 120, ease: 'linear' } },
  };

  const motion = computed<FeedMotion>(() => (reduced.value || !animate.value) ? still : fade);
  return { row: motion, gift: motion };
}
