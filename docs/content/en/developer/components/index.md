---
title: "Shared UI components and composition"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
translationRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
order: 290
---

# Shared UI components and composition {#shared-ui-components-and-composition}

## Goal and prerequisites {#goal-and-prerequisites}

Compose a feature from the existing UI kit while preserving interaction and localization. Use Vue's installed workspace toolchain and read the [frontend map](../frontend.md). This page introduces the kit; each component's source, props, tests and dedicated family reference remain authoritative for its exact API.

## Find the correct primitive {#find-the-correct-primitive}

`src/web/shared/ui/` contains buttons, text/date/color fields, text areas, checkboxes, switches, combo boxes, tabs, tables, forms, dialogs, status, alerts and structural controls. Design tokens live in `src/web/app/styles/tokens.css`; base and responsive styles compose the application. Use tokens for spacing, colors, type and focus treatment rather than creating a second theme in each feature.

1. Find an existing feature with the interaction you need.
2. Read the primitive's `defineProps`, slots, emitted/model values and test before wrapping it.
3. Keep domain validation, saved state and host calls in their owning feature/entity.
4. Pass accessible labels and control attributes through the primitive's supported forwarding contract.

A script-setup component can use the actual button API:

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
</script>

<template>
  <BaseButton variant="primary" type="submit">{{ t('common.apply') }}</BaseButton>
</template>
```

Its containing form owns submit behavior and validation. `BaseButton` supports `default`, `primary`, `danger` and `ghost` variants; a destructive color is not a confirmation workflow by itself.

## Expected result and status {#expected-result-and-status}

Shared primitives provide consistent implemented keyboard, focus and ARIA behavior. Composition still needs review: placeholders are not labels, icon-only controls need names, disabled states must reflect current operation ownership, and asynchronous errors must remain visible. The development UI-kit route is a demonstrator, not a separate product feature or a backend acceptance test.

## Verify and recover {#verify-and-recover}

Run `bun scripts/task.ts test web BaseButton`, the changed component's focused test and `bun scripts/task.ts frontend-lint`. Check narrow width, keyboard navigation and translated labels. If an attribute lands on a wrapper instead of the input, inspect `shared/lib/use-split-attrs.ts` and the control test before bypassing the primitive. Consult [accessibility](../i18n-accessibility.md) and the [UI source](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui) when adapting a family reference example.

## Interactive family references {#families}

- [Buttons and actions](buttons.md)
- [Menus](menus.md)
- [TextField and SpinBox](fields.md)
- [ComboBox and SearchSelect](selects.md)
- [Switches](switches.md)
- [Forms](forms.md)
- [Tabs and toolbars](tabs.md)
- [Lists and tables](collections.md)
- [Filters and selection](filters.md)
- [Alerts, empty states and status](feedback.md)
- [Panels and dialogs](panels.md)
- [FloatingPanel](floating.md)
- [Splitter and MasterDetail](layouts.md)
- [JSON view and diff](json.md)
- [Import and review](import-review.md)
- [Cloud review patterns](cloud-patterns.md)
- [Overlay patterns](overlay-patterns.md)
