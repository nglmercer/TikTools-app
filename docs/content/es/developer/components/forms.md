---
title: "Formularios"
docId: "developer.components.forms"
translationKey: "developer.components.forms"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
translationRevision: "c72fb4fbd4db7dcd7ed98117e1582566931de7380c03d3dbdb53217c3a5d5678"
---

# Formularios {#forms}

## Objetivo y requisitos {#purpose}

Asocia etiquetas, ayudas y errores al control real. Las filas admiten asociación automática y atributos explícitos del slot. Usa el workspace Vue existente con su proveedor i18n. La vista monta los componentes reales en un iframe aislado de la aplicación; el tema documental no carga el CSS global del escritorio.

## Probar el ejemplo {#example}

1. Carga la vista o abre la demo completa.
2. Cambia el selector para inspeccionar carga, vacío, error, desactivado, texto largo y móvil cuando correspondan. Un estado sin presentación distinta no añade comportamiento artificial al componente.
3. Usa Tab, Enter, Escape y flechas según el control. Comprueba nombre accesible, foco visible y retorno del foco al cerrar menús o diálogos.
4. Cambia un valor o ejecuta su acción y revisa la selección o resultado. Recargar o Reiniciar restaura el estado de la demo.

<DemoFrame scenario="settings" locale="es" route="/components/forms" title="Ejemplo de Formularios" />

## API {#api}

Las propiedades, valores por defecto, eventos, slots y modelos siguientes se generan desde el componente. Los tipos importados conservan su referencia al código sin duplicarse manualmente. Los atributos no declarados siguen el contrato de reenvío del componente; revisa el código antes de asociar un evento nativo a un contenedor.

<!--@include: ../../../../.generated/components/forms.es.md-->

## Código ejecutable {#source}

Este archivo se compila para la vista y se incluye como snippet. El estado pequeño pertenece al ejemplo. La lógica de cuentas, plugins y host permanece en features/entities, fuera de shared/ui.

<<< ../../../../examples/forms.vue

## Accesibilidad y recuperación {#accessibility}

Proporciona etiqueta visible o aria-label, conserva el manejo por teclado y evita depender solo del color. Prueba un ancho reducido y etiquetas traducidas largas. En formularios asocia controlAttrs de la fila al input real para conservar errores y estado obligatorio. Un control desactivado no explica por sí mismo un fallo: muestra estado cercano y recuperación.

Si falla la vista, ciérrala y ábrela y luego reinicia su instancia. La API estática y el código siguen siendo útiles sin demo. Los diálogos nativos, servicios externos y funciones del sistema requieren pruebas separadas de escritorio. [Volver a la guía de componentes](index.md).

## Detalles de operación {#operational-details}

`BaseSchemaForm` controla modelo JSON, no persistencia. Pasa schema, `uiHints.fields`, `fieldOptions`, `fieldErrors`, `suggestionsFor` y `openMediaPicker` opcionales. Un hint usa `showIf:{key,equals,notEquals}` para visibilidad; `advanced` agrupa campos opcionales. El propietario carga opciones dinámicas y conserva selección. Errores van por campo; seleccionar medios reales necesita picker suministrado. El ejemplo combina textarea/media condicional y grupo. Los valores aceptados actualizan modelo; al guardar sigue validación de dominio/host.

`BaseFormRow` infiere un input salvo `group=true`. El slot publica `controlAttrs:{id,aria-describedby,aria-invalid,aria-required}` para el control real. Con varios usa `group`, nombra cada hijo y no repitas el mismo id; ayuda/error se asocian al grupo. `useSplitAttrs` dirige class/style al contenedor y demás atributos/listeners al control. File upload acepta un archivo y comprueba tamaño/tipo configurados; falla limpiando modelo y mostrando error. Seleccionar archivo no es subirlo remotamente.
