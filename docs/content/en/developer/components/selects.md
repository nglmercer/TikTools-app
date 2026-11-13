---
title: "ComboBox and SearchSelect"
docId: "developer.components.selects"
translationKey: "developer.components.selects"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
translationRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
---

# ComboBox and SearchSelect {#combobox-and-searchselect}

## Purpose and prerequisites {#purpose}

Choose a stable value from labelled options. SearchSelect searches labels and values; an unknown selected value is displayed as-is. Use the existing Vue workspace and its i18n provider. The preview mounts the real shared components in an isolated application frame; the documentation theme does not load desktop global CSS.

## Try the example {#example}

1. Load the preview or open the full demo.
2. Change the state selector to inspect loading, empty, error, disabled, long text and mobile presentation where applicable. A state without a distinct rendering does not add an artificial behavior to the component.
3. Use Tab, Enter, Escape and arrow keys according to the control. Check its accessible name, visible focus, and return focus when closing a popup.
4. Change a value or invoke its action and inspect the resulting selection or output. Refresh or Reset restores demo state.

<DemoFrame scenario="settings" locale="en" route="/components/selects" title="ComboBox and SearchSelect example" />

## API {#api}

The following props, defaults, events, slots and model declarations are generated from the component source. Imported types remain linked to that source instead of being manually duplicated. Undeclared attributes follow the component's forwarding contract; review its source before attaching a native event to a wrapper.

<!--@include: ../../../../.generated/components/selects.en.md-->

## Executable source {#source}

This exact file is both compiled for the preview and included as the snippet. Its small state belongs to the example. Account, plugin and host logic stays in features/entities rather than shared UI.

<<< ../../../../examples/selects.vue

## Accessibility and recovery {#accessibility}

Use a visible label or aria-label, preserve keyboard operation and avoid relying on color alone. Test at a narrow width and with long translated labels. For forms, bind the row's controlAttrs to the actual input so errors and required state remain associated. A disabled control cannot explain a failure on its own; show an adjacent status and a recovery action.

If a preview fails, close and reopen it, then Reset its own demo instance. Review the static API and source even when the demo is unavailable. Native dialogs, external services and operating-system integration require separate desktop tests. [Return to the component guide](index.md).

## Operational details {#operational-details}

Supply the selected option even while loading so its label can render; an absent option falls back to the raw model value. `search` emits the query for the owner’s optional remote lookup, while internal matching still searches label and value. Arrow keys/Home/End move among enabled options; Enter selects the active row. With `allowCustom`, committing a typed value trims it; Escape cancels that edit. Check an unloaded selected ID, disabled rows and a custom value in addition to a successful selection.

## Operational details {#keyboard-contract}

Review [keyboard, dialog exceptions and tooltip naming](../i18n-accessibility.md#operational-details) for this family. Check focus entry/return in the complete composed flow, including nonclosable dialogs and unmounted draft editors.
