---
title: "Listas y tablas"
docId: "developer.components.collections"
translationKey: "developer.components.collections"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45424c522635bc6271707b05d18c8d206878efbb9f9657dab9a9e69f3876832e"
translationRevision: "94876bb6f71badabcdcb82ff41c657515f43ccb2929a653fa1f85a9a9306b36d"
---

# Listas y tablas {#lists-and-tables}

## Objetivo y requisitos {#purpose}

Selecciona claves estables y activa registros con teclado o puntero. Ordenar cambia el modelo; el responsable entrega las filas ordenadas. Usa el workspace Vue existente con su proveedor i18n. La vista monta los componentes reales en un iframe aislado de la aplicación; el tema documental no carga el CSS global del escritorio.

## Probar el ejemplo {#example}

1. Carga la vista o abre la demo completa.
2. Cambia el selector para inspeccionar carga, vacío, error, desactivado, texto largo y móvil cuando correspondan. Un estado sin presentación distinta no añade comportamiento artificial al componente.
3. Usa Tab, Enter, Escape y flechas según el control. Comprueba nombre accesible, foco visible y retorno del foco al cerrar menús o diálogos.
4. Cambia un valor o ejecuta su acción y revisa la selección o resultado. Recargar o Reiniciar restaura el estado de la demo.

<DemoFrame scenario="settings" locale="es" route="/components/collections" title="Ejemplo de Listas y tablas" />

## API {#api}

Las propiedades, valores por defecto, eventos, slots y modelos siguientes se generan desde el componente. Los tipos importados conservan su referencia al código sin duplicarse manualmente. Los atributos no declarados siguen el contrato de reenvío del componente; revisa el código antes de asociar un evento nativo a un contenedor.

<!--@include: ../../../../.generated/components/collections.es.md-->

## Código ejecutable {#source}

Este archivo se compila para la vista y se incluye como snippet. El estado pequeño pertenece al ejemplo. La lógica de cuentas, plugins y host permanece en features/entities, fuera de shared/ui.

<<< ../../../../examples/collections.vue

## Accesibilidad y recuperación {#accessibility}

Proporciona etiqueta visible o aria-label, conserva el manejo por teclado y evita depender solo del color. Prueba un ancho reducido y etiquetas traducidas largas. En formularios asocia controlAttrs de la fila al input real para conservar errores y estado obligatorio. Un control desactivado no explica por sí mismo un fallo: muestra estado cercano y recuperación.

Si falla la vista, ciérrala y ábrela y luego reinicia su instancia. La API estática y el código siguen siendo útiles sin demo. Los diálogos nativos, servicios externos y funciones del sistema requieren pruebas separadas de escritorio. [Volver a la guía de componentes](index.md).

## Detalles de operación {#operational-details}

El ejemplo conserva estado en el padre. Sort de tabla es modelo enlazado que ordena tres filas mediante computed; el encabezado cambia orden y selección conserva ID. Los editores de pestañas guardan borradores fuera del hijo con key: escribir, cambiar y volver conserva cada borrador. Pestaña/diálogo no los persisten. Elige estado padre, contenido montado o KeepAlive expresamente.
