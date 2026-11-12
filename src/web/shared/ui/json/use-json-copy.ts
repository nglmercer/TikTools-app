import { onBeforeUnmount, ref } from 'vue';

/** Copy the full document independently of the currently rendered, bounded view. */
export function useJsonCopy(textSource: () => string, onCopy: (text: string) => void) {
  const copied = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onBeforeUnmount(() => clearTimeout(timer));

  async function copy(): Promise<void> {
    const text = textSource();
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      try { fallbackCopy(text); } catch { return; }
    }
    copied.value = true;
    clearTimeout(timer);
    timer = setTimeout(() => { copied.value = false; }, 1600);
    onCopy(text);
  }

  return { copied, copy };
}

function fallbackCopy(text: string): void {
  const area = document.createElement('textarea');
  area.value = text;
  area.setAttribute('readonly', '');
  area.style.position = 'fixed';
  area.style.opacity = '0';
  document.body.append(area);
  area.select();
  try { document.execCommand('copy'); } finally { area.remove(); }
}
