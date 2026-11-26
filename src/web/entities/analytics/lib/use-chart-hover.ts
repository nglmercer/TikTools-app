import { ref, type Ref } from 'vue';

/**
 * Pointer-to-category tracking for the hand-rolled SVG charts: maps the
 * pointer x position onto the nearest slot index. Charts are categorical
 * (days, hours, weekdays), so the vertical position is irrelevant — any
 * hover over the plot snaps to the closest slot, which is how every
 * analytics tool behaves.
 */
export function useChartHover(box: { width: number; pad: number }, slots: Ref<number>) {
  const hoverIndex = ref<number | null>(null);

  function onPointermove(event: PointerEvent): void {
    const svg = event.currentTarget as SVGSVGElement | null;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    if (rect.width <= 0) return;
    // Scale the rendered position back into viewBox units — the svg is
    // responsive, so client pixels are not chart units.
    const x = ((event.clientX - rect.left) / rect.width) * box.width;
    const inner = box.width - box.pad * 2;
    const count = slots.value;
    if (count <= 0 || inner <= 0) {
      hoverIndex.value = null;
      return;
    }
    const ratio = (x - box.pad) / inner;
    const index = Math.round(ratio * (count - 1));
    hoverIndex.value = Math.max(0, Math.min(count - 1, index));
  }

  function onPointerleave(): void {
    hoverIndex.value = null;
  }

  /** Horizontal center of a slot in viewBox units. */
  function slotX(index: number): number {
    const count = slots.value;
    const inner = box.width - box.pad * 2;
    if (count <= 1) return box.pad + inner / 2;
    return box.pad + (inner * index) / (count - 1);
  }

  return { hoverIndex, onPointermove, onPointerleave, slotX };
}
