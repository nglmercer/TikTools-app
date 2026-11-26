import { describe, expect, it } from 'vitest';
import BaseDateText from '../../../../../../src/web/shared/ui/date-text/BaseDateText.vue';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';

const DAY = 86_400;
/** 2026-11-23T00:00:00Z */
const NOV_23 = Date.UTC(2026, 10, 23) / 1_000;
const NOV_26 = NOV_23 + 3 * DAY;

describe('BaseDateText', () => {
  it('renders full and compact variants of a single date', () => {
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: NOV_23, utc: true } });
    const full = wrapper.get('.base-date-text__full');
    const compact = wrapper.get('.base-date-text__compact');
    expect(full.text()).toContain('23');
    expect(full.text()).toContain('2026');
    expect(compact.text()).toContain('23');
    // Compact drops the year when it matches the current one; 2026 may or may
    // not be current, but it must never exceed the full form's information.
    expect(compact.text().length).toBeLessThanOrEqual(full.text().length);
  });

  it('pins day-based values to UTC so the date never shifts', () => {
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: NOV_23, utc: true } });
    expect(wrapper.get('time').attributes('datetime')).toBe(new Date(NOV_23 * 1_000).toISOString());
    expect(wrapper.get('.base-date-text__full').text()).toContain('23');
  });

  it('renders a span across two days with an en dash', () => {
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: NOV_23, endValue: NOV_26, utc: true } });
    expect(wrapper.text()).toContain('–');
    expect(wrapper.findAll('time')).toHaveLength(0);
  });

  it('collapses a same-day span into a single time element', () => {
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: NOV_23, endValue: NOV_23, utc: true } });
    expect(wrapper.findAll('time')).toHaveLength(1);
    expect(wrapper.text()).not.toContain('–');
  });

  it('shows only the clock in compact datetime when the day is today', () => {
    const todayNoon = Math.floor(Date.now() / 1_000) - 3600;
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: todayNoon, mode: 'datetime' } });
    const compact = wrapper.get('.base-date-text__compact').text();
    expect(compact).toMatch(/^\d{2}:\d{2}$/);
    // The full variant keeps the date.
    expect(wrapper.get('.base-date-text__full').text().length).toBeGreaterThan(compact.length);
  });

  it('keeps date and clock for datetimes of other days', () => {
    const wrapper = mountWithPlugins(BaseDateText, { props: { value: NOV_23 + 81_540, mode: 'datetime' } });
    expect(wrapper.get('.base-date-text__full').text()).toContain('·');
  });
});
