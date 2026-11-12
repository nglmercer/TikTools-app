import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import BaseJsonView from './BaseJsonView.vue';

enableAutoUnmount(afterEach);

const value = { name: 'Gift alert', enabled: true, count: 3, none: null, list: [1, { deep: 'x' }], empty: {} };
const pretty = JSON.stringify(value, null, 2);

describe('BaseJsonView code mode', () => {
  it('keeps the pretty-printed JSON as the exact text of one pre element', () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value } });
    expect(wrapper.findAll('pre')).toHaveLength(1);
    expect(wrapper.get('pre').element.textContent).toBe(pretty);
  });

  it('colours tokens by kind with token classes', () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value } });
    expect(wrapper.get('.json-token--key').text()).toBe('"name"');
    expect(wrapper.get('.json-token--string').text()).toBe('"Gift alert"');
    expect(wrapper.find('.json-token--boolean').text()).toBe('true');
    expect(wrapper.find('.json-token--number').text()).toBe('3');
    expect(wrapper.find('.json-token--null').text()).toBe('null');
    expect(wrapper.find('.json-token--punct').exists()).toBe(true);
  });

  it('exposes a focusable, labelled scroll region with a max height', () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, maxHeight: '12rem' } });
    const region = wrapper.get('[role="region"]');
    expect(region.attributes('tabindex')).toBe('0');
    expect(region.attributes('aria-label')).toBe('JSON document');
    expect(region.attributes('style')).toContain('--base-json-max-height: 12rem');
    const custom = mountWithPlugins(BaseJsonView, { props: { value, label: 'Saved source', maxHeight: 200 } });
    expect(custom.get('[role="region"]').attributes('aria-label')).toBe('Saved source');
    expect(custom.get('[role="region"]').attributes('style')).toContain('200px');
  });

  it('numbers lines through data attributes and toggles soft wrap', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, lineNumbers: true } });
    const lines = wrapper.findAll('.base-json__line');
    expect(lines).toHaveLength(pretty.split('\n').length);
    expect(lines[0]!.attributes('data-line')).toBe('1');
    expect(lines.at(-1)!.attributes('data-line')).toBe(String(lines.length));
    expect(wrapper.get('pre').classes()).toContain('base-json__code--numbers');
    expect(wrapper.get('pre').classes()).toContain('base-json__code--wrap');
    const toggle = wrapper.get('button[aria-label="Wrap lines"]');
    expect(toggle.attributes('aria-pressed')).toBe('true');
    await toggle.trigger('click');
    expect(wrapper.get('pre').classes()).not.toContain('base-json__code--wrap');
  });

  it('copies the full pretty text and confirms it', async () => {
    vi.useFakeTimers();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value } });
    await wrapper.get('button[aria-label="Copy JSON"]').trigger('click');
    await vi.waitFor(() => expect(wrapper.get('[role="status"]').text()).toBe('Copied'));
    expect(writeText).toHaveBeenCalledWith(pretty);
    expect(wrapper.emitted('copy')![0]).toEqual([pretty]);
    vi.advanceTimersByTime(2000);
    await wrapper.vm.$nextTick();
    expect(wrapper.get('[role="status"]').text()).toBe('');
    vi.useRealTimers();
  });

  it('can hide the copy button', () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, copyable: false } });
    expect(wrapper.find('button[aria-label="Copy JSON"]').exists()).toBe(false);
  });

  it('truncates large documents with a notice but still copies everything', async () => {
    const big = Array.from({ length: 500 }, (_, index) => index);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value: big, maxLines: 20 } });
    expect(wrapper.findAll('.base-json__line')).toHaveLength(20);
    expect(wrapper.text()).toContain('only the first part is shown');
    await wrapper.get('button[aria-label="Copy JSON"]').trigger('click');
    expect(writeText).toHaveBeenCalledWith(JSON.stringify(big, null, 2));
  });

  it('pretty-prints a raw source and shows invalid text as is', () => {
    const ok = mountWithPlugins(BaseJsonView, { props: { source: '{"a":1}' } });
    expect(ok.get('pre').element.textContent).toBe('{\n  "a": 1\n}');
    const bad = mountWithPlugins(BaseJsonView, { props: { source: '{"a": oops', switchable: true, mode: 'tree' } });
    expect(bad.get('pre').element.textContent).toBe('{"a": oops');
    expect(bad.find('.base-json__tree').exists()).toBe(false);
    expect(bad.find('button[aria-label="Tree view"]').exists()).toBe(false);
  });

  it('renders a 5000-line document without stalling', () => {
    const big = { items: Array.from({ length: 1250 }, (_, index) => ({ id: index, name: `item ${index}`, on: true })) };
    const started = performance.now();
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value: big, lineNumbers: true } });
    expect(wrapper.findAll('.base-json__line')).toHaveLength(5000);
    expect(performance.now() - started).toBeLessThan(5000);
  });
});

describe('BaseJsonView tree mode', () => {
  it('shows a collapsible tree with counts for collapsed nodes', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, mode: 'tree' } });
    expect(wrapper.find('pre').exists()).toBe(false);
    const root = wrapper.get('.base-json__toggle');
    expect(root.attributes('aria-expanded')).toBe('true');
    const list = wrapper.findAll('.base-json__toggle').find((button) => button.text().includes('"list"'))!;
    expect(list.attributes('aria-expanded')).toBe('false');
    expect(list.text()).toBe('"list": [2]');
    expect(wrapper.text()).toContain('"empty": {}');
    await list.trigger('click');
    expect(list.attributes('aria-expanded')).toBe('true');
    const inner = wrapper.findAll('.base-json__toggle').find((button) => button.text().startsWith('1:'))!;
    expect(inner.text()).toBe('1: {1}');
  });

  it('expands and collapses everything with the toolbar buttons', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, mode: 'tree' } });
    await wrapper.get('button[aria-label="Expand all"]').trigger('click');
    expect(wrapper.text()).toContain('"deep": "x"');
    expect(wrapper.findAll('.base-json__toggle').every((button) => button.attributes('aria-expanded') === 'true')).toBe(true);
    await wrapper.get('button[aria-label="Collapse all"]').trigger('click');
    const toggles = wrapper.findAll('.base-json__toggle');
    expect(toggles).toHaveLength(1);
    expect(toggles[0]!.text()).toBe('{6}');
    expect(toggles[0]!.attributes('aria-expanded')).toBe('false');
  });

  it('switches between code and tree when switchable', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value, switchable: true } });
    const code = wrapper.get('button[aria-label="Code view"]');
    const tree = wrapper.get('button[aria-label="Tree view"]');
    expect(code.attributes('aria-pressed')).toBe('true');
    expect(wrapper.find('pre').exists()).toBe(true);
    await tree.trigger('click');
    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.find('.base-json__tree').exists()).toBe(true);
    expect(tree.attributes('aria-pressed')).toBe('true');
    await code.trigger('click');
    expect(wrapper.find('pre').exists()).toBe(true);
  });

  it('retains expanded nodes when the tree is unmounted for code or invalid source', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { source: '{"nested":{"value":7}}', mode: 'tree', switchable: true } });
    const nested = () => wrapper.findAll('.base-json__toggle').find((button) => button.text().includes('"nested"'))!;
    await nested().trigger('click');
    expect(wrapper.text()).toContain('"value": 7');
    await wrapper.get('button[aria-label="Code view"]').trigger('click');
    await wrapper.get('button[aria-label="Tree view"]').trigger('click');
    expect(nested().attributes('aria-expanded')).toBe('true');
    await wrapper.setProps({ source: 'invalid JSON' });
    expect(wrapper.get('pre').text()).toBe('invalid JSON');
    await wrapper.setProps({ source: '{"nested":{"value":8}}' });
    expect(nested().attributes('aria-expanded')).toBe('true');
    expect(wrapper.text()).toContain('"value": 8');
  });

  it('hides the switch unless asked for', () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value } });
    expect(wrapper.find('button[aria-label="Tree view"]').exists()).toBe(false);
  });

  it('caps the depth the tree can open', async () => {
    const deep = { a: { b: { c: { d: 1 } } } };
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value: deep, mode: 'tree', treeDepth: 2 } });
    await wrapper.get('button[aria-label="Expand all"]').trigger('click');
    expect(wrapper.text()).toContain('"b": {1}');
    expect(wrapper.text()).not.toContain('"c"');
  });

  it('shows a notice when a huge array is cut', async () => {
    const wrapper = mountWithPlugins(BaseJsonView, { props: { value: Array.from({ length: 3000 }, (_, index) => index), mode: 'tree' } });
    expect(wrapper.text()).toContain('500 more entries not shown');
    expect(wrapper.text()).toContain('only the first part is shown');
  });
});
