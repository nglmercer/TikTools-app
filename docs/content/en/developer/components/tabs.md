---
title: "Tabs and toolbars"
docId: "developer.components.tabs"
translationKey: "developer.components.tabs"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6341644dd3da8b4d7a5ef9a079a884908997a5a3f8f65618746cdb8d9db81ad8"
translationRevision: "6341644dd3da8b4d7a5ef9a079a884908997a5a3f8f65618746cdb8d9db81ad8"
---

# Tabs and toolbars {#tabs-and-toolbars}

## Purpose and prerequisites {#purpose}

Switch between related views without losing drafts. Give tab groups accessible labels and keep toolbar commands keyboard reachable. Use the existing Vue workspace and its i18n provider. The preview mounts the real shared components in an isolated application frame; the documentation theme does not load desktop global CSS.

## Try the example {#example}

1. Load the preview or open the full demo.
2. Change the state selector to inspect loading, empty, error, disabled, long text and mobile presentation where applicable. A state without a distinct rendering does not add an artificial behavior to the component.
3. Use Tab, Enter, Escape and arrow keys according to the control. Check its accessible name, visible focus, and return focus when closing a popup.
4. Change a value or invoke its action and inspect the resulting selection or output. Refresh or Reset restores demo state.

<DemoFrame scenario="settings" locale="en" route="/components/tabs" title="Tabs and toolbars example" />

## API {#api}

The following props, defaults, events, slots and model declarations are generated from the component source. Imported types remain linked to that source instead of being manually duplicated. Undeclared attributes follow the component's forwarding contract; review its source before attaching a native event to a wrapper.

<!--@include: ../../../../.generated/components/tabs.en.md-->

## Executable source {#source}

This exact file is both compiled for the preview and included as the snippet. Its small state belongs to the example. Account, plugin and host logic stays in features/entities rather than shared UI.

<<< ../../../../examples/tabs.vue

## Accessibility and recovery {#accessibility}

Use a visible label or aria-label, preserve keyboard operation and avoid relying on color alone. Test at a narrow width and with long translated labels. For forms, bind the row's controlAttrs to the actual input so errors and required state remain associated. A disabled control cannot explain a failure on its own; show an adjacent status and a recovery action.

If a preview fails, close and reopen it, then Reset its own demo instance. Review the static API and source even when the demo is unavailable. Native dialogs, external services and operating-system integration require separate desktop tests. [Return to the component guide](index.md).

## Operational details {#operational-details}

The runnable snippet keeps state in its parent. Table sort is a bound model used by a computed ordering of three rows; changing the header changes row order while selection keeps the stable ID. Tab editors store drafts outside the keyed child, so typing, switching and returning preserves each draft. A tab or dialog does not persist drafts itself. Choose parent state, mounted content or KeepAlive deliberately.

## Operational details {#keyboard-contract}

Review [keyboard, dialog exceptions and tooltip naming](../i18n-accessibility.md#operational-details) for this family. Check focus entry/return in the complete composed flow, including nonclosable dialogs and unmounted draft editors.
