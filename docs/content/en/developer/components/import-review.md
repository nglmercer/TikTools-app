---
title: "Import and review"
docId: "developer.components.import-review"
translationKey: "developer.components.import-review"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "0d2900f8be78c415674d80b76f48d296eaef1e0bbfe0d5e3754c5f6f904bdc38"
translationRevision: "0d2900f8be78c415674d80b76f48d296eaef1e0bbfe0d5e3754c5f6f904bdc38"
---

# Import and review {#import-and-review}

## Purpose and prerequisites {#purpose}

Read a bounded file or pasted document, stage parsed records and ask for explicit import. The shell never executes imported content. Use the existing Vue workspace and its i18n provider. The preview mounts the real shared components in an isolated application frame; the documentation theme does not load desktop global CSS.

## Try the example {#example}

1. Load the preview or open the full demo.
2. Change the state selector to inspect loading, empty, error, disabled, long text and mobile presentation where applicable. A state without a distinct rendering does not add an artificial behavior to the component.
3. Use Tab, Enter, Escape and arrow keys according to the control. Check its accessible name, visible focus, and return focus when closing a popup.
4. Change a value or invoke its action and inspect the resulting selection or output. Refresh or Reset restores demo state.

<DemoFrame scenario="settings" locale="en" route="/components/import-review" title="Import and review example" />

## API {#api}

The following props, defaults, events, slots and model declarations are generated from the component source. Imported types remain linked to that source instead of being manually duplicated. Undeclared attributes follow the component's forwarding contract; review its source before attaching a native event to a wrapper.

<!--@include: ../../../../.generated/components/import-review.en.md-->

## Executable source {#source}

This exact file is both compiled for the preview and included as the snippet. Its small state belongs to the example. Account, plugin and host logic stays in features/entities rather than shared UI.

<<< ../../../../examples/import-review.vue

## Accessibility and recovery {#accessibility}

Use a visible label or aria-label, preserve keyboard operation and avoid relying on color alone. Test at a narrow width and with long translated labels. For forms, bind the row's controlAttrs to the actual input so errors and required state remain associated. A disabled control cannot explain a failure on its own; show an adjacent status and a recovery action.

If a preview fails, close and reopen it, then Reset its own demo instance. Review the static API and source even when the demo is unavailable. Native dialogs, external services and operating-system integration require separate desktop tests. [Return to the component guide](index.md).
