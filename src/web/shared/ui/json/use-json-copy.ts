import { onBeforeUnmount, ref, type Ref } from 'vue';

const COPY_FEEDBACK_MS = 1600;

/**
 * Shared clipboard copy with fallback and timed feedback. The feedback state
 * resets after `duration` and never outlives the component. Keyed variant:
 * pass a key per row so independent rows track their own copied state.
 */
export function useCopyFeedback<K = true>(duration = COPY_FEEDBACK_MS) {
  const copied = ref(false) as Ref<boolean>;
  const copiedKey = ref<K | null>(null) as Ref<K | null>;
  let timer: ReturnType<typeof setTimeout> | undefined;
  onBeforeUnmount(() => clearTimeout(timer));

  async function writeClipboard(text: string): Promise<boolean> {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      try { fallbackCopy(text); return true; } catch { return false; }
    }
  }

  async function copy(text: string): Promise<void> {
    if (!await writeClipboard(text)) return;
    copied.value = true;
    clearTimeout(timer);
    timer = setTimeout(() => { copied.value = false; }, duration);
  }

  async function copyKeyed(key: K, text: string): Promise<void> {
    if (!await writeClipboard(text)) return;
    copiedKey.value = key;
    clearTimeout(timer);
    timer = setTimeout(() => { copiedKey.value = null; }, duration);
  }

  return { copied, copiedKey, copy, copyKeyed };
}

/** Copy the full document independently of the currently rendered, bounded view. */
export function useJsonCopy(textSource: () => string, onCopy: (text: string) => void) {
  const { copied, copy: copyText } = useCopyFeedback();
  async function copy(): Promise<void> {
    const text = textSource();
    await copyText(text);
    if (copied.value) onCopy(text);
  }
  return { copied, copy };
}

export function fallbackCopy(text: string): void {
  const area = document.createElement('textarea');
  area.value = text;
  area.setAttribute('readonly', '');
  area.style.position = 'fixed';
  area.style.opacity = '0';
  document.body.append(area);
  area.select();
  try { document.execCommand('copy'); } finally { area.remove(); }
}
