import { expect, test } from 'bun:test';

import {
  ICONS,
  IconChat,
  IconClose,
  IconHeart,
  IconSparkles,
  resolveIcon,
  SvgIcon,
  type IconName,
} from './icons.tsx';
import { EVENT_PRESENTATION, presentationForEvent, workflowEventChoices } from './event-icons.ts';
import { BUILTIN_EVENT_TYPES } from '../../../automation/contracts/events.ts';

type VNodeProps = Record<string, unknown> & {
  width?: unknown;
  height?: unknown;
  class?: unknown;
  stroke?: unknown;
  fill?: unknown;
  'aria-hidden'?: unknown;
};

function propsOf(vnode: unknown): VNodeProps {
  return ((vnode as { props?: VNodeProps }).props ?? {}) as VNodeProps;
}

test('SvgIcon renders an accessible svg that inherits currentColor', () => {
  const props = propsOf(SvgIcon({ children: [] }));
  expect(props.width).toBe(16);
  expect(props.height).toBe(16);
  expect(props.stroke).toBe('currentColor');
  expect(props['aria-hidden']).toBe('true');
});

test('icon size and class propagate through the shared primitive', () => {
  const vnode = IconChat({ size: 18, className: 'my-icon' }) as { type?: unknown; props?: VNodeProps };
  expect(vnode.type).toBe(SvgIcon);
  expect(vnode.props?.size).toBe(18);
  expect(vnode.props?.className).toBe('my-icon');
});

test('filled icons delegate with the filled flag, strokes stay currentColor', () => {
  const filled = propsOf(IconHeart({}));
  expect(filled.filled).toBe(true);
  const svgProps = propsOf(SvgIcon({ filled: true, children: [] }));
  expect(svgProps.fill).toBe('currentColor');
  expect(svgProps.stroke).toBe('none');
  expect(svgProps['aria-hidden']).toBe('true');
});

test('sparkles icon is a multi-glint composition, not a single star', () => {
  const vnode = IconSparkles({}) as { children?: unknown[] };
  const children = Array.isArray(vnode.children) ? vnode.children : [];
  expect(children.length).toBeGreaterThan(1);
});

test('every IconName resolves to a component delegating to SvgIcon', () => {
  const names = Object.keys(ICONS) as IconName[];
  expect(names.length).toBeGreaterThan(30);
  for (const name of names) {
    const vnode = ICONS[name]({ size: 20 }) as { type?: unknown; props?: VNodeProps };
    expect(vnode.type).toBe(SvgIcon);
    expect(vnode.props?.size).toBe(20);
  }
});

test('resolveIcon falls back to a neutral glyph for unknown names', () => {
  expect(resolveIcon('no-such-icon')).toBe(ICONS.dot);
  expect(resolveIcon('close')).toBe(IconClose);
});

test('event presentation covers every built-in event type', () => {
  for (const eventType of BUILTIN_EVENT_TYPES) {
    const presentation = EVENT_PRESENTATION[eventType];
    expect(ICONS[presentation.icon]).toBeDefined();
    expect(presentation.label.default.length).toBeGreaterThan(0);
  }
  expect(workflowEventChoices()).toHaveLength(BUILTIN_EVENT_TYPES.length);
});

test('presentationForEvent falls back for unknown plugin event types', () => {
  const presentation = presentationForEvent('my-plugin.custom');
  expect(presentation.icon).toBe('plugin');
  expect(presentation.label.default).toBe('my-plugin.custom');
});
