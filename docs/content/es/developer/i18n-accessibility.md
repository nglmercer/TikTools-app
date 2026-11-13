---
title: "Internacionalización y accesibilidad"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "56df24ca7f2e599f17ab161e54cee92bc41aeed3788789e949bee20cbe227713"
translationRevision: "56df24ca7f2e599f17ab161e54cee92bc41aeed3788789e949bee20cbe227713"
order: 120
---

# Internacionalización y accesibilidad {#internationalization-and-accessibility}

## Objetivo y requisitos {#goal-and-prerequisites}

Añade pantallas comprensibles en los idiomas admitidos y utilizables con teclado, tacto y tecnologías de asistencia. Lee las [capas frontend](frontend.md) y usa la interfaz compartida. Los idiomas de la aplicación y las ediciones inglesa/española de esta documentación son sistemas distintos.

## Añadir contenido localizado {#add-localized-content}

El catálogo está en `src/web/shared/i18n/`. Inglés es la referencia tipada; español, portugués y francés se cargan bajo demanda. `activateLocale` carga los mensajes y actualiza `document.documentElement.lang`. Guarda la preferencia mediante el almacén existente, sin crear otra clave de almacenamiento local.

1. Añade la clave semántica al catálogo inglés y a sus módulos equivalentes en los otros idiomas.
2. Conserva identificadores API, rutas de eventos y recursos guardados; traduce sus etiquetas visibles.
3. Usa interpolación para textos variables y las utilidades compartidas de formato.
4. Para descriptores de plugins, emplea `{default,i18key}` y los catálogos integrados por el host. Nunca interpreta como HTML los textos traducidos o proporcionados por plugins.

## Conservar la semántica de los controles {#preserve-control-semantics}

Prefiere `BaseTextField`, `BaseFormRow`, `BaseDialog`, pestañas y componentes de estado. Un texto de ejemplo no reemplaza una etiqueta accesible. Los botones de icono necesitan nombre localizado. La validación debe identificar el campo y exponer su invalidez; el color no basta para comunicar errores. Conserva retorno del foco, Escape, selección mediante teclado y contornos visibles.

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

En revisión manual, recorre todo con Tab, abre/cierra el diálogo, cambia idioma mientras está abierto y repite con ancho estrecho. Las acciones de estado visibles al pasar el cursor también deben aparecer con foco y estar disponibles al tocar.

## Resultado esperado y estado {#expected-result-and-status}

El tipado y las comprobaciones de catálogos detectan claves ausentes; los controles compartidos implementan teclado y ARIA. Esto no certifica automáticamente la accesibilidad de cada pantalla compuesta. Rankings sigue oculto aunque tenga traducciones.

## Recuperación {#recovery}

Si aparece texto sin traducir, revisa carga diferida y nombre exacto de la clave antes de añadir inglés dentro del componente. Si desaparece el foco, inspecciona el control receptor de atributos y el destino de retorno. La [implementación i18n](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) y las pruebas definen el comportamiento actual; [componentes](components/index.md) presenta las entradas compartidas.
