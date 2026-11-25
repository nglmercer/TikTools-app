import { expect, test } from 'vitest';
import { resolveWidgetDesign, styleVariables } from '../../../../src/widgets/sdk/template.ts';
import { widgetTemplates } from '../../../../src/widgets/sdk/templates.ts';
import { textDefaults, type TextField } from '../../../../src/widgets/sdk/text.ts';

test('editor styling rejects CSS injection and bounds geometry', () => {
  expect(styleVariables({ background: '#112233', textColor: '#ffffffff', accent: 'url(https://example.com)', radius: 100 })).toEqual({
    '--widget-background': '#112233', '--widget-textColor': '#ffffffff', '--widget-radius': '48px',
  });
  expect(styleVariables({ radius: NaN })).toEqual({});
});

test('style variables expose card, avatar, and badge tokens', () => {
  expect(styleVariables({
    borderColor: '#2a2a33', borderWidth: 2, shadow: false, opacity: 80,
    padding: 24, gap: 12, align: 'center', width: 400,
    avatar: { visible: false, size: 64, radius: 12, borderColor: '#ffffff', borderWidth: 3 },
    badge: { color: '#00dce8', fontSize: 14, fontWeight: 800, letterSpacing: 2 },
  })).toEqual({
    '--widget-borderColor': '#2a2a33',
    '--widget-borderWidth': '2px',
    '--widget-shadow': 'none',
    '--widget-opacity': '0.8',
    '--widget-padding': '24px',
    '--widget-gap': '12px',
    '--widget-align': 'center',
    '--widget-align-items': 'center',
    '--widget-width': '400px',
    '--widget-avatar-display': 'none',
    '--widget-avatar-size': '64px',
    '--widget-avatar-radius': '12px',
    '--widget-avatar-borderColor': '#ffffff',
    '--widget-avatar-borderWidth': '3px',
    '--widget-badge-color': '#00dce8',
    '--widget-badge-size': '14px',
    '--widget-badge-weight': '800',
    '--widget-badge-spacing': '2px',
  });
});

test('style variables omit unset optionals and invalid values', () => {
  expect(styleVariables({})).toEqual({});
  expect(styleVariables({ shadow: true, align: 'diagonal' as never, width: NaN })).toEqual({});
  expect(styleVariables({ avatar: { visible: true }, badge: {} })).toEqual({});
});

test('style variables expose bounded portable motion tokens', () => {
  expect(styleVariables({
    motion: { preset: 'slide-left', enterMs: 5000, exitMs: -20, easing: 'spring', staggerMs: 40 },
  })).toEqual({
    '--widget-motion-enter-transform': 'translateX(24px) scale(0.98)',
    '--widget-motion-exit-transform': 'translateX(16px) scale(0.98)',
    '--widget-motion-enter-duration': '2000ms',
    '--widget-motion-exit-duration': '0ms',
    '--widget-motion-enter-easing': 'cubic-bezier(0.2, 0.9, 0.25, 1.2)',
    '--widget-motion-exit-easing': 'cubic-bezier(0.5, 0, 0.75, 0.4)',
    '--widget-motion-stagger': '40ms',
  });
  expect(styleVariables({ motion: { preset: 'none' } })).toEqual({
    '--widget-motion-enter-transform': 'none',
    '--widget-motion-exit-transform': 'none',
    '--widget-motion-enter-duration': '0ms',
    '--widget-motion-exit-duration': '0ms',
  });
});

test('every template exposes one schema for editor and preview defaults', () => {
  for (const [kind, template] of Object.entries(widgetTemplates)) {
    const defaults = Object.keys(textDefaults[kind as keyof typeof textDefaults]) as TextField[];
    // Rankings expose only the title line their widget renders (see rankingSchema).
    expect(template.schema.textFields).toEqual(kind === 'ranking' ? ['title'] : defaults);
    expect(template.schema.defaultDesign.background).toBeTruthy();
    expect(template.schema.defaultDesign.accent).toBeTruthy();
    expect(resolveWidgetDesign(template, {}).text).toBeUndefined();
    expect(resolveWidgetDesign(template, { text: { title: 'Custom' } }).text?.title).toBe('Custom');
  }
});
