import { describe, expect, it } from 'vitest';
import { ref } from 'vue';
import BaseDateRange from '../../../../../../src/web/shared/ui/date-range/BaseDateRange.vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';

function mountRange() {
  const start = ref('2026-11-20');
  const end = ref('2026-11-26');
  const wrapper = mountWithPlugins(BaseDateRange, {
    props: {
      fromLabel: 'From',
      toLabel: 'To',
      start: start.value,
      end: end.value,
      'onUpdate:start': (value: string) => (start.value = value),
      'onUpdate:end': (value: string) => (end.value = value),
    },
  });
  return { wrapper, start, end };
}

describe('BaseDateRange', () => {
  it('cross-constrains the fields so start never passes end', () => {
    const { wrapper } = mountRange();
    const [from, to] = wrapper.findAll('input');
    expect(from!.attributes('max')).toBe('2026-11-26');
    expect(to!.attributes('min')).toBe('2026-11-20');
  });

  it('updates the models when the fields change', async () => {
    const { wrapper, start, end } = mountRange();
    const [from, to] = wrapper.findAll('input');
    await from!.setValue('2026-11-01');
    await to!.setValue('2026-11-30');
    expect(start.value).toBe('2026-11-01');
    expect(end.value).toBe('2026-11-30');
  });

  it('renders the range labels', () => {
    const { wrapper } = mountRange();
    expect(wrapper.text()).toContain('From');
    expect(wrapper.text()).toContain('To');
  });
});
