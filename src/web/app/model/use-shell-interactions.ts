import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue';
import { onPullToRefresh, onSwipe } from '@/shared/lib/touch-gestures.ts';

/** Instantiate once per shell: media, keyboard and touch listeners share its lifetime. */
export function useShellInteractions(sidebarCollapsed: Readonly<Ref<boolean>>, toggleDesktopSidebar: () => void) {
  const drawerOpen = ref(false);
  const isNarrow = ref(false);
  let narrowQuery: MediaQueryList | null = null;

  function onNarrowChange(event: MediaQueryListEvent): void {
    isNarrow.value = event.matches;
    if (!event.matches) drawerOpen.value = false;
  }

  const sidebarExpanded = computed(() => (isNarrow.value ? drawerOpen.value : !sidebarCollapsed.value));
  const sidebarRail = computed(() => !isNarrow.value && sidebarCollapsed.value);
  function toggleSidebar(): void {
    if (isNarrow.value) drawerOpen.value = !drawerOpen.value;
    else toggleDesktopSidebar();
  }

  function closeDrawer(): void {
    drawerOpen.value = false;
  }

  /* ---- Touch gestures (mobile) ---- */
  const contentRef = ref<HTMLElement | null>(null);
  const pullDistance = ref(0);
  const pullArmed = ref(false);
  const gestureDisposers: Array<() => void> = [];

  function reloadApp(): void {
    pullDistance.value = 0;
    window.location.reload();
  }

  function installGestures(): void {
    // Swipe in from the left edge opens the drawer on narrow layouts.
    gestureDisposers.push(onSwipe(window, {
      edge: 24,
      enabled: () => isNarrow.value && !drawerOpen.value,
      onSwipe: (direction) => { if (direction === 'right') drawerOpen.value = true; },
    }));
    // Swiping left on the open drawer (or its scrim) closes it.
    gestureDisposers.push(onSwipe(window, {
      enabled: () => isNarrow.value && drawerOpen.value,
      onSwipe: (direction) => { if (direction === 'left') drawerOpen.value = false; },
    }));
    if (contentRef.value) {
      gestureDisposers.push(onPullToRefresh(contentRef.value, {
        onPull: (distance, armed) => {
          pullDistance.value = distance;
          pullArmed.value = armed;
        },
        onRefresh: reloadApp,
      }));
    }
  }

  function onShellKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && isNarrow.value && drawerOpen.value && !event.defaultPrevented) {
      drawerOpen.value = false;
    }
  }

  onMounted(() => {
    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      narrowQuery = window.matchMedia('(width < 900px)');
      isNarrow.value = narrowQuery.matches;
      if (typeof narrowQuery.addEventListener === 'function') narrowQuery.addEventListener('change', onNarrowChange);
      else narrowQuery.addListener(onNarrowChange as (event: MediaQueryListEvent) => void);
    }
    window.addEventListener('keydown', onShellKeydown);
    installGestures();
  });

  onBeforeUnmount(() => {
    if (narrowQuery) {
      if (typeof narrowQuery.removeEventListener === 'function') narrowQuery.removeEventListener('change', onNarrowChange);
      else narrowQuery.removeListener(onNarrowChange as (event: MediaQueryListEvent) => void);
      narrowQuery = null;
    }
    window.removeEventListener('keydown', onShellKeydown);
    for (const dispose of gestureDisposers.splice(0)) dispose();
  });

  return { drawerOpen, isNarrow, sidebarExpanded, sidebarRail, contentRef, pullDistance, pullArmed, toggleSidebar, closeDrawer };
}
