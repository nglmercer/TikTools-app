---
title: "Pestañas y barras"
docId: "developer.components.tabs"
translationKey: "developer.components.tabs"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6341644dd3da8b4d7a5ef9a079a884908997a5a3f8f65618746cdb8d9db81ad8"
translationRevision: "a5283afd2e5632b7ac114f014d60b461b03f2ec16c54d5a3b18a91042fe6c4f0"
---

# Pestañas y barras {#tabs-and-toolbars}

## Objetivo y requisitos {#purpose}

Cambia entre vistas relacionadas sin perder borradores. Etiqueta los grupos de pestañas y conserva acceso por teclado a sus comandos. Usa el workspace Vue existente con su proveedor i18n. La vista monta los componentes reales en un iframe aislado de la aplicación; el tema documental no carga el CSS global del escritorio.

## Probar el ejemplo {#example}

1. Carga la vista o abre la demo completa.
2. Cambia el selector para inspeccionar carga, vacío, error, desactivado, texto largo y móvil cuando correspondan. Un estado sin presentación distinta no añade comportamiento artificial al componente.
3. Usa Tab, Enter, Escape y flechas según el control. Comprueba nombre accesible, foco visible y retorno del foco al cerrar menús o diálogos.
4. Cambia un valor o ejecuta su acción y revisa la selección o resultado. Recargar o Reiniciar restaura el estado de la demo.

<DemoFrame scenario="settings" locale="es" route="/components/tabs" title="Ejemplo de Pestañas y barras" />

## API {#api}

Las propiedades, valores por defecto, eventos, slots y modelos siguientes se generan desde el componente. Los tipos importados conservan su referencia al código sin duplicarse manualmente. Los atributos no declarados siguen el contrato de reenvío del componente; revisa el código antes de asociar un evento nativo a un contenedor.

<!--@include: ../../../../.generated/components/tabs.es.md-->

## Código ejecutable {#source}

Este archivo se compila para la vista y se incluye como snippet. El estado pequeño pertenece al ejemplo. La lógica de cuentas, plugins y host permanece en features/entities, fuera de shared/ui.

<<< ../../../../examples/tabs.vue

## Accesibilidad y recuperación {#accessibility}

Proporciona etiqueta visible o aria-label, conserva el manejo por teclado y evita depender solo del color. Prueba un ancho reducido y etiquetas traducidas largas. En formularios asocia controlAttrs de la fila al input real para conservar errores y estado obligatorio. Un control desactivado no explica por sí mismo un fallo: muestra estado cercano y recuperación.

Si falla la vista, ciérrala y ábrela y luego reinicia su instancia. La API estática y el código siguen siendo útiles sin demo. Los diálogos nativos, servicios externos y funciones del sistema requieren pruebas separadas de escritorio. [Volver a la guía de componentes](index.md).

## Detalles de operación {#operational-details}

El ejemplo conserva estado en el padre. Sort de tabla es modelo enlazado que ordena tres filas mediante computed; el encabezado cambia orden y selección conserva ID. Los editores de pestañas guardan borradores fuera del hijo con key: escribir, cambiar y volver conserva cada borrador. Pestaña/diálogo no los persisten. Elige estado padre, contenido montado o KeepAlive expresamente.

## Detalles de operación {#keyboard-contract}

Revisa [teclado, excepciones del diálogo y nombres/tooltip](../i18n-accessibility.md#operational-details). Comprueba foco inicial/retorno en el flujo completo, incluidos diálogos no cerrables y borradores desmontados.
