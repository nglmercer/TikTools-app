---
title: "Lists and tables"
docId: "developer.components.collections"
translationKey: "developer.components.collections"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45424c522635bc6271707b05d18c8d206878efbb9f9657dab9a9e69f3876832e"
translationRevision: "45424c522635bc6271707b05d18c8d206878efbb9f9657dab9a9e69f3876832e"
---

# Lists and tables {#lists-and-tables}

## Purpose and prerequisites {#purpose}

Select stable keys and activate records with keyboard or pointer. Sorting changes the model; the owner supplies ordered rows. Use the existing Vue workspace and its i18n provider. The preview mounts the real shared components in an isolated application frame; the documentation theme does not load desktop global CSS.

## Try the example {#example}

1. Load the preview or open the full demo.
2. Change the state selector to inspect loading, empty, error, disabled, long text and mobile presentation where applicable. A state without a distinct rendering does not add an artificial behavior to the component.
3. Use Tab, Enter, Escape and arrow keys according to the control. Check its accessible name, visible focus, and return focus when closing a popup.
4. Change a value or invoke its action and inspect the resulting selection or output. Refresh or Reset restores demo state.

<DemoFrame scenario="settings" locale="en" route="/components/collections" title="Lists and tables example" />

## API {#api}

The following props, defaults, events, slots and model declarations are generated from the component source. Imported types remain linked to that source instead of being manually duplicated. Undeclared attributes follow the component's forwarding contract; review its source before attaching a native event to a wrapper.

<!--@include: ../../../../.generated/components/collections.en.md-->

## Executable source {#source}

This exact file is both compiled for the preview and included as the snippet. Its small state belongs to the example. Account, plugin and host logic stays in features/entities rather than shared UI.

<<< ../../../../examples/collections.vue

## Accessibility and recovery {#accessibility}

Use a visible label or aria-label, preserve keyboard operation and avoid relying on color alone. Test at a narrow width and with long translated labels. For forms, bind the row's controlAttrs to the actual input so errors and required state remain associated. A disabled control cannot explain a failure on its own; show an adjacent status and a recovery action.

If a preview fails, close and reopen it, then Reset its own demo instance. Review the static API and source even when the demo is unavailable. Native dialogs, external services and operating-system integration require separate desktop tests. [Return to the component guide](index.md).

## Operational details {#operational-details}

The runnable snippet keeps state in its parent. Table sort is a bound model used by a computed ordering of three rows; changing the header changes row order while selection keeps the stable ID. Tab editors store drafts outside the keyed child, so typing, switching and returning preserves each draft. A tab or dialog does not persist drafts itself. Choose parent state, mounted content or KeepAlive deliberately.
