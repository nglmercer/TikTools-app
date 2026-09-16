import { expect, test } from 'bun:test';
import { createSSRApp, h } from 'vue';
import { renderToString } from 'vue/server-renderer';

import {
  ICONS,
  IconChat,
  IconClose,
  IconHeart,
  IconSparkles,
  readIconName,
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

/**
 * Bun's JSX transform passes component children as arrays where Vite emits
 * slot functions; silence only that transform artifact so render assertions
 * stay readable.
 */
function quietApp(root: Parameters<typeof h>[0], props?: Parameters<typeof h>[1], slots?: Parameters<typeof h>[2]) {
  const app = createSSRApp({ render: () => h(root, props ?? {}, slots ?? {}) });
  app.config.warnHandler = (message) => {
    if (typeof message === 'string' && message.includes('Non-function value encountered for default slot')) return;
    console.warn(message);
  };
  return app;
}

test('SvgIcon renders slot children into an accessible svg that inherits currentColor', async () => {
  const app = quietApp(SvgIcon, { size: 18, className: 'my-icon' }, {
    default: () => h('path', { d: 'M0 0h1v1H0z' }),
  });
  const html = await renderToString(app);
  expect(html).toContain('<svg');
  expect(html).toContain('width="18"');
  expect(html).toContain('height="18"');
  expect(html).toContain('stroke="currentColor"');
  expect(html).toContain('aria-hidden="true"');
  expect(html).toContain('class="my-icon"');
  expect(html).toContain('<path d="M0 0h1v1H0z">');
});

test('SvgIcon filled variant uses currentColor fills without a stroke', async () => {
  const app = quietApp(SvgIcon, { filled: true }, {
    default: () => h('circle', { cx: 12, cy: 12, r: 8 }),
  });
  const html = await renderToString(app);
  expect(html).toContain('fill="currentColor"');
  expect(html).toContain('stroke="none"');
  expect(html).toContain('<circle');
});

test('icon size and class propagate through the shared primitive', () => {
  const vnode = IconChat({ size: 18, className: 'my-icon' }) as { type?: unknown; props?: VNodeProps };
  expect(vnode.type).toBe(SvgIcon);
  expect(vnode.props?.size).toBe(18);
  expect(vnode.props?.className).toBe('my-icon');
});

test('filled icons delegate with the filled flag', () => {
  const filled = propsOf(IconHeart({}));
  expect(filled.filled).toBe(true);
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

test('every registered icon renders a non-empty svg', async () => {
  const drawable = /<(path|polygon|polyline|line|circle|rect|ellipse)\b/;
  for (const name of Object.keys(ICONS) as IconName[]) {
    const app = quietApp(ICONS[name], { size: 20 });
    const html = await renderToString(app);
    expect(html).toContain('<svg');
    expect(html).toContain('width="20"');
    expect(html.includes('aria-hidden="true"')).toBe(true);
    expect(drawable.test(html)).toBe(true);
  }
});

test('resolveIcon falls back to a neutral glyph for unknown names', () => {
  expect(resolveIcon('no-such-icon')).toBe(ICONS.dot);
  expect(resolveIcon('close')).toBe(IconClose);
});

test('readIconName whitelists registry names from untrusted JSON', () => {
  for (const name of Object.keys(ICONS) as IconName[]) expect(readIconName(name)).toBe(name);
  expect(readIconName('close')).toBe('close');
  expect(readIconName('no-such-icon')).toBeUndefined();
  expect(readIconName('constructor')).toBeUndefined();
  expect(readIconName('__proto__')).toBeUndefined();
  expect(readIconName('hasOwnProperty')).toBeUndefined();
  expect(readIconName(42)).toBeUndefined();
  expect(readIconName(null)).toBeUndefined();
  expect(readIconName(undefined)).toBeUndefined();
  expect(readIconName({})).toBeUndefined();
  expect(readIconName(['close'])).toBeUndefined();
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
