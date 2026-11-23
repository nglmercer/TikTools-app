import { computed } from 'vue';
import { useReducedMotion, type MotionVariants } from '@vueuse/motion';

export type FeedMotion = MotionVariants<string>;

/** Entry motion for feed rows. Resolved once per list so each card does not
 *  register its own media-query listener. */
export function useFeedMotion() {
  const reduced = useReducedMotion();

  const still: FeedMotion = {
    initial: { opacity: 1 },
    enter: { opacity: 1 },
  };

  const row = computed<FeedMotion>(() => reduced.value ? still : {
    initial: { opacity: 0, y: 8 },
    enter: { opacity: 1, y: 0, transition: { duration: 180, ease: 'easeOut' } },
  });

  const gift = computed<FeedMotion>(() => reduced.value ? still : {
    initial: { opacity: 0, scale: 0.96 },
    enter: { opacity: 1, scale: 1, transition: { duration: 260, ease: 'easeOut' } },
  });

  return { row, gift };
}
