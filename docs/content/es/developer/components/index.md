---
title: "Componentes compartidos y composición de interfaz"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7049a03b9a3b437dc969ee83208bbaac8521bd55bf0e557e96a185ffdfaff9f6"
translationRevision: "fbeddfaa0a9f244ace38ddaab9e1c4d7b73b495d444eb92037cffca17a83c468"
order: 290
---

# Componentes compartidos y composición de interfaz {#shared-ui-components-and-composition}

## Objetivo y requisitos {#goal-and-prerequisites}

Compón funcionalidades con el kit existente conservando interacción y localización. Usa las herramientas Vue del workspace y lee el [mapa frontend](../frontend.md). Esta página presenta el kit; código, propiedades, pruebas y referencia de cada familia mantienen autoridad sobre su API exacta.

## Encontrar el componente adecuado {#find-the-correct-primitive}

`src/web/shared/ui/` contiene botones, campos de texto/fecha/color, áreas de texto, casillas, interruptores, selectores, pestañas, tablas, formularios, diálogos, estados, alertas y estructura. Los tokens están en `src/web/app/styles/tokens.css`; estilos base y adaptables componen la aplicación. Utilízalos para espaciado, color, tipografía y foco, evitando otro tema por funcionalidad.

1. Busca una funcionalidad existente con la interacción necesaria.
2. Lee `defineProps`, slots, valores de modelo/eventos y pruebas antes de envolver el componente.
3. Conserva validación de dominio, estado guardado y llamadas al host en su funcionalidad/entidad.
4. Pasa etiquetas y atributos accesibles mediante el contrato de distribución admitido.

Un componente script-setup puede usar la API real del botón:

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

El formulario contenedor controla envío y validación. `BaseButton` admite `default`, `primary`, `danger` y `ghost`; un color destructivo no constituye confirmación por sí mismo.

## Resultado esperado y estado {#expected-result-and-status}

Los componentes compartidos implementan comportamiento consistente de teclado, foco y ARIA. La composición necesita revisión: ejemplos de texto no son etiquetas, iconos requieren nombres, deshabilitar debe reflejar la operación actual y los errores asíncronos deben seguir visibles. La ruta de desarrollo del kit demuestra componentes; no es otra funcionalidad del producto ni una prueba de aceptación del backend.

## Verificación y recuperación {#verify-and-recover}

Ejecuta `bun scripts/task.ts test web BaseButton`, la prueba dirigida del componente modificado y `bun scripts/task.ts frontend-lint`. Comprueba anchura estrecha, teclado y etiquetas traducidas. Si un atributo termina en el contenedor en vez del input, revisa `shared/lib/use-split-attrs.ts` y sus pruebas antes de omitir el componente. Consulta [accesibilidad](../i18n-accessibility.md) y el [código de interfaz](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui) al adaptar ejemplos de familias.

## Referencias interactivas por familia {#families}

- [Botones y acciones](buttons.md)
- [Menús](menus.md)
- [TextField y SpinBox](fields.md)
- [ComboBox y SearchSelect](selects.md)
- [Interruptores](switches.md)
- [Formularios](forms.md)
- [Pestañas y barras](tabs.md)
- [Listas y tablas](collections.md)
- [Filtros y selección](filters.md)
- [Alertas, vacíos y estados](feedback.md)
- [Paneles y diálogos](panels.md)
- [FloatingPanel](floating.md)
- [Splitter y MasterDetail](layouts.md)
- [Vista y diff JSON](json.md)
- [Importación y revisión](import-review.md)
- [Patrones de revisión Cloud](cloud-patterns.md)
- [Patrones de overlays](overlay-patterns.md)

## Detalles de operación {#operational-details}

Las familias generadas cubren deliberadamente primitivas públicas. [Formularios](forms.md) incluye SchemaForm/SettingsRow; [campos](fields.md), textarea, slider, radio, tags, fecha, color y archivo único; [menús](menus.md), MenuBar/ContextMenu; [feedback](feedback.md), Notification. Iconos, SchemaFieldRows y piezas internas no son contratos públicos independientes. Lee bindings de slots además de props/modelos. Una firma generada no garantiza validación ni accesibilidad.
