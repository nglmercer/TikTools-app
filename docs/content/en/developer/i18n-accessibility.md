---
title: "Internationalization and accessibility"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2c74fcefd1d28353b1135c547b2e3e6ae852af8b683350011732d10c364f9323"
translationRevision: "2c74fcefd1d28353b1135c547b2e3e6ae852af8b683350011732d10c364f9323"
order: 120
---

# Internationalization and accessibility {#internationalization-and-accessibility}

## Goal and prerequisites {#goal-and-prerequisites}

Add a screen that remains understandable in supported locales and usable by keyboard, touch and assistive technology. Read [frontend layers](frontend.md) and use the existing shared UI. Runtime application locales and the published documentation editions are separate systems.

## Add localized content {#add-localized-content}

The application catalog lives in `src/web/shared/i18n/`. English is the typed fallback; All other supported catalogs load lazily. `activateLocale` loads the chosen messages and updates `document.documentElement.lang`. Store a locale preference through the existing locale store rather than writing another local-storage key.

1. Add the semantic message key to the English catalog and equivalent supported catalogs/modules.
2. Keep API IDs, event paths and stored resource identifiers stable; translate their visible labels.
3. Use interpolation for variable text and shared internationalization helpers for display values.
4. For plugin descriptors, use the existing `{default,i18key}` localized text contract and host-merged catalogs. Never render translated or plugin-provided strings as HTML.

## Preserve control semantics {#preserve-control-semantics}

Prefer `BaseTextField`, `BaseFormRow`, `BaseDialog`, tabs and status components. A placeholder does not replace an accessible label. Icon buttons need a localized name. Validation must identify the field and expose invalid state; do not communicate failure only through color. Preserve dialog focus return, Escape behavior, keyboard selection and visible focus outlines.

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

For manual review, tab through the complete flow, open/close the dialog, switch locale while it is open and repeat at narrow width. Status actions that reveal on hover must also reveal on keyboard focus and remain available on touch.

## Expected result and status {#expected-result-and-status}

Catalog typing and parity checks catch missing keys; shared controls provide implemented keyboard and ARIA behavior. They do not constitute a blanket accessibility certification of every composed screen. Rankings remains hidden even if translated strings exist.

## Recovery {#recovery}

If untranslated text appears, check lazy catalog loading and exact key spelling before adding English fallbacks in components. If focus disappears, inspect the control receiving forwarded attributes and the dialog's return target. The [i18n implementation](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) and component tests define the current behavior; [components](components/index.md) explains the shared entry points.

## Operational details {#operational-details}

Locale detection is saved `tiktok-live-locale` → `navigator.languages` → English. `normalizeLocale` handles regional tags and `jp` → `ja`. UI codes/format regions are `en/en-US`, `es/es-ES`, `pt/pt-BR`, `fr/fr-FR`, `zh/zh-CN`, `ja/ja-JP`; use `intl.ts` formatCount/formatDate. `activateLocale` loads messages and document lang; `useLocaleStore().setLocale` also persists and stages preferences best-effort. Use the store for a preference control. English is fallback; zh/ja also load host-message modules. To add a locale, update supported-locales, lazy loaders, catalog/host messages, format-region mapping, preference tests, catalog key/placeholder/plural compilation tests, and corresponding docs/site locale routing. Preserve API IDs and parameter names.

Use spacing/type/control/focus/z-layer tokens rather than fixed sizes. Any coarse pointer or viewport under 700px raises controls to 44px; layout navigation changes under 900px and form rows under 600px independently. Reduced motion zeroes durations. Test coarse pointer at desktop width too. The app is dark globally; the light token override is JSON-specific, not a global light-theme feature.

Keyboard map: tabs use arrows/Home/End with automatic selection; selects use arrows/Home/End/Enter and Escape cancellation; menus return focus to their trigger. Dialogs use native showModal focus containment, but `closable=false` blocks Escape and closing unmounts content. Keep drafts with the owner and verify initial/return focus in the composed screen. Icon buttons need `label`; `hint`/tooltip is extra explanation, never their name. Tooltips appear after 500ms on hover/focus, close with Escape and stay in the dialog top layer. Review real keyboard and assistive flows; unit tests do not certify a whole screen.
