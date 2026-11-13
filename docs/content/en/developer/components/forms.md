---
title: "Forms"
docId: "developer.components.forms"
translationKey: "developer.components.forms"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
translationRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
---

# Forms {#forms}

## Purpose and prerequisites {#purpose}

Associate labels, hints and errors with their actual control. Form rows support both automatic association and explicit scoped attributes. Use the existing Vue workspace and its i18n provider. The preview mounts the real shared components in an isolated application frame; the documentation theme does not load desktop global CSS.

## Try the example {#example}

1. Load the preview or open the full demo.
2. Change the state selector to inspect loading, empty, error, disabled, long text and mobile presentation where applicable. A state without a distinct rendering does not add an artificial behavior to the component.
3. Use Tab, Enter, Escape and arrow keys according to the control. Check its accessible name, visible focus, and return focus when closing a popup.
4. Change a value or invoke its action and inspect the resulting selection or output. Refresh or Reset restores demo state.

<DemoFrame scenario="settings" locale="en" route="/components/forms" title="Forms example" />

## API {#api}

The following props, defaults, events, slots and model declarations are generated from the component source. Imported types remain linked to that source instead of being manually duplicated. Undeclared attributes follow the component's forwarding contract; review its source before attaching a native event to a wrapper.

<!--@include: ../../../../.generated/components/forms.en.md-->

## Executable source {#source}

This exact file is both compiled for the preview and included as the snippet. Its small state belongs to the example. Account, plugin and host logic stays in features/entities rather than shared UI.

<<< ../../../../examples/forms.vue

## Accessibility and recovery {#accessibility}

Use a visible label or aria-label, preserve keyboard operation and avoid relying on color alone. Test at a narrow width and with long translated labels. For forms, bind the row's controlAttrs to the actual input so errors and required state remain associated. A disabled control cannot explain a failure on its own; show an adjacent status and a recovery action.

If a preview fails, close and reopen it, then Reset its own demo instance. Review the static API and source even when the demo is unavailable. Native dialogs, external services and operating-system integration require separate desktop tests. [Return to the component guide](index.md).

## Operational details {#operational-details}

`BaseSchemaForm` owns a JSON-object model, not persistence. Pass a JSON schema, `uiHints.fields`, `fieldOptions`, `fieldErrors`, optional `suggestionsFor` and `openMediaPicker`. A field hint uses `showIf:{key,equals,notEquals}` for conditional visibility; `advanced` groups optional fields. Dynamic options belong to the owner, which must load them and preserve the selected value. Errors are keyed by field; a media control needs a supplied picker to select real media. The runnable example combines a conditional textarea/media form and a control group. Accepted values update the model; domain/host validation still runs when saving.

`BaseFormRow` infers one input unless `group` is true. Its default slot exposes `controlAttrs:{id,aria-describedby,aria-invalid,aria-required}` for the actual control. For multiple controls use `group`, name each child independently and do not spread the same id onto every child; group help/error is associated with the group. `useSplitAttrs` routes class/style to the wrapper and other attributes/listeners to the control. File upload accepts one file and checks configured size/type; failure clears the model and exposes an error. Upload selection is not a remote upload.
