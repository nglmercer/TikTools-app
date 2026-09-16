import { expect, test } from 'bun:test';
import { createSSRApp, h, type VNodeChild } from 'vue';
import { renderToString } from 'vue/server-renderer';

import { defineVueComponent } from './component.ts';

/**
 * Regression coverage for the shared slot bridge. Bun cannot import `.vue`
 * SFCs, so these tests exercise `defineVueComponent` with the exact runtime
 * prop declarations `Tooltip.vue` and `Modal.vue` use. Cases pass children
 * via `h()` slots objects, which is precisely what `@vitejs/plugin-vue-jsx`
 * emits for `<Tooltip text="x"><button/>…` in the shipped bundle. (Bun's own
 * JSX transform passes children as arrays instead, so JSX is deliberately
 * avoided here.)
 */

function quietApp(root: Parameters<typeof h>[0], props?: Parameters<typeof h>[1], slots?: Parameters<typeof h>[2]) {
  const app = createSSRApp({ render: () => h(root, props ?? {}, slots ?? {}) });
  app.config.warnHandler = () => {};
  return app;
}

type TooltipShapeProps = {
  text: string;
  position?: string;
  wide?: boolean;
  disabled?: boolean;
  children: VNodeChild;
};

/** Mirrors Tooltip.vue: declared `children` prop wrapping slot content in a trigger. */
const TooltipShape = defineVueComponent<TooltipShapeProps>(
  ['text', 'position', 'wide', 'disabled', 'children'],
  (props) => () => h('span', { class: 'ui-tooltip-trigger' }, props.children as never),
);

type ModalShapeProps = {
  title: string;
  children?: VNodeChild;
  footer?: VNodeChild;
  className?: string;
};

/** Mirrors Modal.vue: declared `children`/`footer`/`className` props. */
const ModalShape = defineVueComponent<ModalShapeProps>(
  ['title', 'children', 'footer', 'className'],
  (props) => () => h(
    'div',
    { class: `ui-modal-card ${props.className ?? ''}`.trim() },
    [
      h('h2', props.title),
      props.children ? h('div', { class: 'ui-modal-card__body' }, props.children as never) : null,
      props.footer ? h('footer', { class: 'ui-modal-card__footer' }, props.footer as never) : null,
    ],
  ),
);

test('declared-but-absent children resolve to the default slot (Tooltip shape)', async () => {
  const app = quietApp(TooltipShape, { text: 'Chats' }, {
    default: () => h('button', { type: 'button' }, 'filter'),
  });
  const html = await renderToString(app);
  expect(html).toContain('class="ui-tooltip-trigger"');
  expect(html).toContain('<button type="button">filter</button>');
});

test('explicit children prop wins over the default slot', async () => {
  const app = quietApp(
    TooltipShape,
    { text: 'Chats', children: h('em', 'explicit') },
    { default: () => h('button', 'slot') },
  );
  const html = await renderToString(app);
  expect(html).toContain('<em>explicit</em>');
  expect(html).not.toContain('<button');
});

test('missing children render an empty trigger without crashing', async () => {
  const app = quietApp(TooltipShape, { text: 'Chats' });
  const html = await renderToString(app);
  expect(html).toContain('class="ui-tooltip-trigger"');
});

test('Modal shape renders slot children plus an explicit footer prop', async () => {
  const app = quietApp(
    ModalShape,
    { title: 'Confirm', footer: h('button', 'ok') },
    { default: () => h('p', 'body text') },
  );
  const html = await renderToString(app);
  expect(html).toContain('<h2>Confirm</h2>');
  expect(html).toContain('<div class="ui-modal-card__body"><p>body text</p></div>');
  expect(html).toContain('<footer class="ui-modal-card__footer"><button>ok</button></footer>');
});

test('declared-but-absent className falls back to the fallthrough class', async () => {
  const app = quietApp(ModalShape, { title: 'T', class: 'from-attrs' });
  const html = await renderToString(app);
  expect(html).toContain('class="ui-modal-card from-attrs"');
});

test('explicit className wins over the fallthrough class', async () => {
  const app = quietApp(ModalShape, { title: 'T', className: 'explicit', class: 'from-attrs' });
  const html = await renderToString(app);
  expect(html).toContain('class="ui-modal-card explicit"');
});
