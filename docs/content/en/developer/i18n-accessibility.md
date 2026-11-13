---
title: "Internationalization and accessibility"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "febf4065849bc28ea2af3cff90ba3966a454f3dc1e5c98a860291ba716c61c05"
translationRevision: "febf4065849bc28ea2af3cff90ba3966a454f3dc1e5c98a860291ba716c61c05"
order: 120
---

# Internationalization and accessibility

## Goal and prerequisites

Add a screen that remains understandable in supported locales and usable by keyboard, touch and assistive technology. Read [frontend layers](frontend.md) and use the existing shared UI. Runtime application locales and the English/Spanish documentation editions are separate systems.

## Add localized content

The application catalog lives in `src/web/shared/i18n/`. English is the typed fallback; Spanish, Portuguese and French load lazily. `activateLocale` loads the chosen messages and updates `document.documentElement.lang`. Store a locale preference through the existing locale store rather than writing another local-storage key.

1. Add the semantic message key to the English catalog and equivalent supported catalogs/modules.
2. Keep API IDs, event paths and stored resource identifiers stable; translate their visible labels.
3. Use interpolation for variable text and shared internationalization helpers for display values.
4. For plugin descriptors, use the existing `{default,i18key}` localized text contract and host-merged catalogs. Never render translated or plugin-provided strings as HTML.

## Preserve control semantics

Prefer `BaseTextField`, `BaseFormRow`, `BaseDialog`, tabs and status components. A placeholder does not replace an accessible label. Icon buttons need a localized name. Validation must identify the field and expose invalid state; do not communicate failure only through color. Preserve dialog focus return, Escape behavior, keyboard selection and visible focus outlines.

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

For manual review, tab through the complete flow, open/close the dialog, switch locale while it is open and repeat at narrow width. Status actions that reveal on hover must also reveal on keyboard focus and remain available on touch.

## Expected result and status

Catalog typing and parity checks catch missing keys; shared controls provide implemented keyboard and ARIA behavior. They do not constitute a blanket accessibility certification of every composed screen. Rankings remains hidden even if translated strings exist.

## Recovery

If untranslated text appears, check lazy catalog loading and exact key spelling before adding English fallbacks in components. If focus disappears, inspect the control receiving forwarded attributes and the dialog's return target. The [i18n implementation](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) and component tests define the current behavior; [components](components/index.md) explains the shared entry points.
