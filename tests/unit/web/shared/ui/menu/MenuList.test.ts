import { describe, expect, it } from 'vitest';

// Vitest does not inject SFC styles into happy-dom, so assert the responsive
// style contract directly on the component source instead of measuring layout.
const [menuListSource] = Object.values(import.meta.glob('../../../../../../src/web/shared/ui/menu/MenuList.vue', { query: '?raw', import: 'default', eager: true })) as string[];

describe('MenuList responsive styles', () => {
  it('caps the popup to the viewport and scrolls overflowing entries', () => {
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*max-width:\s*min\(360px,\s*calc\(100vw - 2rem\)\)/);
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*max-height:/);
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*overflow-y:\s*auto/);
  });

  it('wraps long labels instead of clipping them', () => {
    expect(menuListSource).toMatch(/\.menu-list__label\s*\{[^}]*white-space:\s*normal/);
    expect(menuListSource).toMatch(/\.menu-list__label\s*\{[^}]*overflow-wrap:\s*anywhere/);
    expect(menuListSource).not.toMatch(/\.menu-list__item\s*\{[^}]*white-space:\s*nowrap/);
  });
});
