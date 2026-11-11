import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import BaseJsonDiff from './BaseJsonDiff.vue';

enableAutoUnmount(afterEach);

const before = { id: 'a', name: 'Old name', enabled: true, tags: ['x'] };
const after = { id: 'a', name: 'New name', enabled: true, tags: ['x', 'y'] };

describe('BaseJsonDiff unified', () => {
  it('shows removed and added lines with +/- markers, line numbers and screen reader text', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before, after } });
    const removed = wrapper.findAll('.base-json-diff__line--removed');
    const added = wrapper.findAll('.base-json-diff__line--added');
    expect(removed).toHaveLength(2);
    expect(added).toHaveLength(3);
    expect(removed[0]!.get('.base-json-diff__marker').text()).toContain('-');
    expect(removed[0]!.get('.base-json-diff__marker').text()).toContain('Removed line');
    expect(added[0]!.get('.base-json-diff__marker').text()).toContain('+');
    expect(added[0]!.get('.base-json-diff__marker').text()).toContain('Added line');
    const numbers = removed[0]!.findAll('.base-json-diff__number').map((n) => n.text());
    expect(numbers).toEqual(['3', '']);
    expect(added[0]!.findAll('.base-json-diff__number').map((n) => n.text())).toEqual(['', '3']);
    expect(wrapper.get('.base-json-diff__stats').text()).toBe('1 added, 0 removed, 2 changed');
  });

  it('marks the changed words inside a changed line pair and keeps the rest of the line plain', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before, after, context: 10 } });
    expect(wrapper.findAll('.base-json-diff__word--removed').map((w) => w.text())).toEqual(['Old']);
    expect(wrapper.findAll('.base-json-diff__word--added').map((w) => w.text())).toEqual(['New', ',']);
    const line = wrapper.get('.base-json-diff__line--added');
    expect(line.text()).toContain('"name": "New name",');
  });

  it('collapses long unchanged runs into a button that expands them', async () => {
    const list = Array.from({ length: 60 }, (_, index) => index);
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before: { list, flag: false }, after: { list, flag: true } } });
    const fold = wrapper.get('.base-json-diff__fold');
    expect(fold.text()).toMatch(/^\d+ unchanged lines$/);
    const hidden = Number(fold.text().match(/\d+/)![0]);
    expect(hidden).toBeGreaterThan(50);
    const visibleBefore = wrapper.findAll('.base-json-diff__line').length;
    expect(visibleBefore).toBeLessThan(15);
    await fold.trigger('click');
    expect(wrapper.find('.base-json-diff__fold').exists()).toBe(false);
    expect(wrapper.findAll('.base-json-diff__line').length).toBe(visibleBefore + hidden);
  });

  it('says so when nothing differs and offers the content behind a fold', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before: { a: [1, 2, 3, 4, 5, 6] }, after: { a: [1, 2, 3, 4, 5, 6] } } });
    expect(wrapper.text()).toContain('No differences');
    expect(wrapper.findAll('.base-json-diff__line--added, .base-json-diff__line--removed')).toHaveLength(0);
  });

  it('diffs against a missing side as all added or all removed', () => {
    const created = mountWithPlugins(BaseJsonDiff, { props: { after: { a: 1 } } });
    expect(created.findAll('.base-json-diff__line--added')).toHaveLength(3);
    expect(created.findAll('.base-json-diff__line--removed')).toHaveLength(0);
    const deleted = mountWithPlugins(BaseJsonDiff, { props: { before: { a: 1 } } });
    expect(deleted.findAll('.base-json-diff__line--removed')).toHaveLength(3);
  });

  it('exposes a focusable region and honours max height and truncation', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, {
      props: { before: Array.from({ length: 100 }, (_, i) => i), after: Array.from({ length: 100 }, (_, i) => i + 1), maxLines: 30, maxHeight: '20rem' },
    });
    const region = wrapper.get('[role="region"]');
    expect(region.attributes('tabindex')).toBe('0');
    expect(region.attributes('aria-label')).toBe('JSON changes');
    expect(region.attributes('style')).toContain('20rem');
    expect(wrapper.text()).toContain('only the first part is shown');
  });
});

describe('BaseJsonDiff side by side', () => {
  it('pairs old and new lines in two columns with their own numbers and headings', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before, after, layout: 'side-by-side', beforeLabel: 'Base', afterLabel: 'Cloud', context: 10 } });
    expect(wrapper.get('.base-json-diff__heads').text()).toBe('BaseCloud');
    const changed = wrapper.findAll('.base-json-diff__split').find((row) => row.text().includes('Old name'))!;
    const halves = changed.findAll('.base-json-diff__half');
    expect(halves).toHaveLength(2);
    expect(halves[0]!.classes()).toContain('base-json-diff__line--removed');
    expect(halves[1]!.classes()).toContain('base-json-diff__line--added');
    expect(halves[0]!.text()).toContain('Old name');
    expect(halves[1]!.text()).toContain('New name');
    expect(halves[0]!.get('.base-json-diff__number').text()).toBe('3');
    expect(halves[0]!.find('.base-json-diff__word--removed').text()).toBe('Old');
    expect(halves[1]!.find('.base-json-diff__word--added').text()).toBe('New');
  });

  it('leaves a blank filler opposite pure additions and removals', () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before: { a: 1 }, after: { a: 1, b: 2 }, layout: 'side-by-side' } });
    const split = wrapper.findAll('.base-json-diff__split').find((row) => row.text().includes('"b"'))!;
    expect(split.findAll('.base-json-diff__half')[0]!.classes()).toContain('base-json-diff__line--empty');
    expect(split.findAll('.base-json-diff__half')[1]!.classes()).toContain('base-json-diff__line--added');
  });

  it('collapses unchanged runs in this layout too', async () => {
    const list = Array.from({ length: 40 }, (_, index) => index);
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before: { list, v: 1 }, after: { list, v: 2 }, layout: 'side-by-side' } });
    const fold = wrapper.get('.base-json-diff__fold');
    const rows = wrapper.findAll('.base-json-diff__split').length;
    await fold.trigger('click');
    expect(wrapper.findAll('.base-json-diff__split').length).toBeGreaterThan(rows + 30);
  });

  it('switches layout from the toolbar when switchable', async () => {
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before, after, switchable: true } });
    expect(wrapper.find('.base-json-diff__unified').exists()).toBe(true);
    await wrapper.get('button[aria-label="Side by side"]').trigger('click');
    expect(wrapper.find('.base-json-diff__split').exists()).toBe(true);
    expect(wrapper.get('button[aria-label="Side by side"]').attributes('aria-pressed')).toBe('true');
    await wrapper.get('button[aria-label="Unified"]').trigger('click');
    expect(wrapper.find('.base-json-diff__unified').exists()).toBe(true);
    expect(mountWithPlugins(BaseJsonDiff, { props: { before, after } }).find('button[aria-label="Unified"]').exists()).toBe(false);
  });

  it('renders a 5000-line diff without stalling', () => {
    const make = (shift: number) => ({ items: Array.from({ length: 900 }, (_, index) => ({ id: index, name: `item ${index % 50 === 0 ? index + shift : index}`, on: true })) });
    const started = performance.now();
    const wrapper = mountWithPlugins(BaseJsonDiff, { props: { before: make(0), after: make(1), layout: 'unified' } });
    expect(wrapper.findAll('.base-json-diff__line--added')).toHaveLength(18);
    expect(performance.now() - started).toBeLessThan(5000);
  });
});
