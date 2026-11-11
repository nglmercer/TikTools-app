import { describe, expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { useUnsavedSelectionGuard } from './use-unsaved-selection-guard.ts';

describe('unsaved selection guard', () => {
  it('runs the change at once when nothing is unsaved', () => {
    const run = vi.fn();
    const { guard, discardOpen } = useUnsavedSelectionGuard(ref(false));
    guard(run);
    expect(run).toHaveBeenCalledTimes(1);
    expect(discardOpen.value).toBe(false);
  });

  it('holds the change behind the confirmation and runs it once when confirmed', () => {
    const run = vi.fn();
    const { guard, discardOpen, confirmDiscard } = useUnsavedSelectionGuard(ref(true));
    guard(run);
    expect(run).not.toHaveBeenCalled();
    expect(discardOpen.value).toBe(true);
    confirmDiscard();
    expect(run).toHaveBeenCalledTimes(1);
    expect(discardOpen.value).toBe(false);
    confirmDiscard();
    expect(run).toHaveBeenCalledTimes(1);
  });

  it('drops a change whose dialog was closed without confirming, so it cannot run later', () => {
    const first = vi.fn();
    const second = vi.fn();
    const dirty = ref(true);
    const { guard, discardOpen, confirmDiscard } = useUnsavedSelectionGuard(dirty);
    guard(first);
    discardOpen.value = false;
    confirmDiscard();
    expect(first).not.toHaveBeenCalled();
    guard(second);
    confirmDiscard();
    expect(second).toHaveBeenCalledTimes(1);
    expect(first).not.toHaveBeenCalled();
  });
});
