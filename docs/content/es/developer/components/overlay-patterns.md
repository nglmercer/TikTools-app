---
title: "Patrones de overlays"
docId: "developer.components.overlay-patterns"
translationKey: "developer.components.overlay-patterns"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a2669421fbbd7518680451321a7764827d1f6fc5334b7aa875dbf1eaf5293f54"
translationRevision: "8cb288c2a9912bb85098fd6e461ed57236e9188de73b031918ac6225945dbb17"
---

# Patrones de overlays {#overlay-patterns}

## Objetivo y requisitos {#purpose}

Inspecciona el diálogo existente de biblioteca, previews animados locales, estados vacío/carga/error y selector de imagen acotado. El ejemplo no sube el archivo seleccionado. El gateway nativo y la conexión con OBS requieren la aplicación instalada. Usa el workspace Vue existente con su proveedor i18n. La vista monta los componentes reales en un iframe aislado de la aplicación; el tema documental no carga el CSS global del escritorio.

## Probar el ejemplo {#example}

1. Carga la vista o abre la demo completa.
2. Cambia el selector para inspeccionar carga, vacío, error, desactivado, texto largo y móvil cuando correspondan. Un estado sin presentación distinta no añade comportamiento artificial al componente.
3. Usa Tab, Enter, Escape y flechas según el control. Comprueba nombre accesible, foco visible y retorno del foco al cerrar menús o diálogos.
4. Cambia un valor o ejecuta su acción y revisa la selección o resultado. Recargar o Reiniciar restaura el estado de la demo.

<DemoFrame scenario="settings" locale="es" route="/components/overlay-patterns" title="Ejemplo de Patrones de overlays" />

## API {#api}

Las propiedades, valores por defecto, eventos, slots y modelos siguientes se generan desde el componente. Los tipos importados conservan su referencia al código sin duplicarse manualmente. Los atributos no declarados siguen el contrato de reenvío del componente; revisa el código antes de asociar un evento nativo a un contenedor.

<!--@include: ../../../../.generated/components/overlay-patterns.es.md-->

## Código ejecutable {#source}

Este archivo se compila para la vista y se incluye como snippet. El estado pequeño pertenece al ejemplo. La lógica de cuentas, plugins y host permanece en features/entities, fuera de shared/ui.

<<< ../../../../examples/overlay-patterns.vue

## Accesibilidad y recuperación {#accessibility}

Proporciona etiqueta visible o aria-label, conserva el manejo por teclado y evita depender solo del color. Prueba un ancho reducido y etiquetas traducidas largas. En formularios asocia controlAttrs de la fila al input real para conservar errores y estado obligatorio. Un control desactivado no explica por sí mismo un fallo: muestra estado cercano y recuperación.

Si falla la vista, ciérrala y ábrela y luego reinicia su instancia. La API estática y el código siguen siendo útiles sin demo. Los diálogos nativos, servicios externos y funciones del sistema requieren pruebas separadas de escritorio. [Volver a la guía de componentes](index.md).
