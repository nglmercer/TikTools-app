import { describe, expect, it } from 'vitest';

// Vitest does not inject SFC styles into happy-dom, so assert the responsive
// style contract directly on the component source instead of measuring layout.
const [menuListSource] = Object.values(
  import.meta.glob<string>('../../../../../../src/web/shared/ui/menu/MenuList.vue', { query: '?raw', import: 'default', eager: true }),
);

describe('MenuList responsive styles', () => {
  it('caps the popup to the viewport and scrolls overflowing entries', () => {
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*max-width:\s*min\(360px,\s*calc\(100vw - 2rem\)\)/);
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*max-height:/);
    expect(menuListSource).toMatch(/\.menu-list\s*\{[^}]*overflow-y:\s*auto/);
  });

  it('wraps long labels instead of clipping them', () => {
    expect(menuListSource).toMatch(/\.menu-list__label\s*\{[^}]*white-space:\s*normal/);
    // `break-word` only breaks overlong words at overflow; `anywhere` broke
    // labels eagerly letter-by-letter whenever a sibling squeezed the row.
    expect(menuListSource).toMatch(/\.menu-list__label\s*\{[^}]*overflow-wrap:\s*break-word/);
    expect(menuListSource).not.toMatch(/\.menu-list__item\s*\{[^}]*white-space:\s*nowrap/);
  });

  it('renders details on their own line so long paths never squeeze the label', () => {
    // Variable-picker paths are long shared-prefix strings; the detail line
    // truncates at the start so the discriminating tail stays visible.
    expect(menuListSource).toMatch(/\.menu-list__text\s*\{[^}]*flex-direction:\s*column/);
    expect(menuListSource).toMatch(/\.menu-list__detail\s*\{[^}]*direction:\s*rtl/);
    expect(menuListSource).toMatch(/\.menu-list__detail\s*\{[^}]*text-overflow:\s*ellipsis/);
  });
});
