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

  const restartBurstTimer = (): void => {
    if (burstTimer !== null) clearTimeout(burstTimer);
    burstTimer = setTimeout(() => {
      animate.value = true;
      burstTimer = null;
    }, BURST_WINDOW_MS);
  };

  watch(events, (list) => {
    // Feed entries are appended in receive order, so the newest rows are at
    // the end. Counting backwards and stopping at the burst threshold avoids
    // a full-buffer scan for every packet during high-rate like storms.
    if (!animate.value) {
      restartBurstTimer();
      return;
    }
    const now = Date.now();
    let recent = 0;
    for (let index = list.length - 1; index >= 0; index -= 1) {
      if (now - list[index]!.receivedAt >= BURST_WINDOW_MS) break;
      recent += 1;
      if (recent > BURST_LIMIT) break;
    }
    if (recent <= BURST_LIMIT) return;
    animate.value = false;
    restartBurstTimer();
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
