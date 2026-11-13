---
title: "Importar plantillas y administrar perfiles"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8f48a8f385f19965c6f887a0fe90ca07dc34d65dd65147d529d356c7d896f0b"
translationRevision: "a8f48a8f385f19965c6f887a0fe90ca07dc34d65dd65147d529d356c7d896f0b"
order: 50
---

# Importar plantillas y administrar perfiles

## Objetivo y requisitos

Reutiliza una regla revisada o un conjunto de reglas sin activar efectos accidentalmente. Necesitas **Automations**, un archivo JSON o elemento de catálogo confiable y los plugins exigidos por sus acciones. Exporta el perfil actual antes de sustituir una configuración estable.

## Elegir la superficie adecuada

Un documento `.tiktemplate.json` describe un evento y sus acciones. Un `.tikprofile.json` agrupa varias plantillas. Abre el diálogo de plantillas desde Automations para consultar plantillas integradas o propias, su biblioteca Cloud o la importación manual. La barra de perfiles permite crear, seleccionar, exportar e importar perfiles manualmente. **Plugins → Explore** busca plugins; las plantillas y perfiles se administran aquí.

## Revisar e importar

1. Abre el diálogo correspondiente y elige un archivo o pega JSON compatible. Comprueba el nombre preparado y su origen antes de continuar.
2. Para una plantilla, inspecciona disparador, tipos de acción y parámetros. Para un perfil, revisa también el número de entradas y los parámetros comunes.
3. Sustituye destinos de ejemplo, rutas de medios e identificadores por valores adecuados a tu instalación. No pegues credenciales en archivos que vayas a compartir.
4. Aplica/importa explícitamente y espera el resultado. Las definiciones importadas permanecen desactivadas. Selecciónalas después y revisa cada acción y evento antes de activar explícitamente los deseados.
5. Comienza con una prueba que solo registre texto o inspecciona primero las definiciones. Las pruebas de HTTP, audio, puntos y plugins pueden ejecutar efectos.
6. Confirma el perfil activo antes del LIVE. Cambiarlo modifica el contexto de biblioteca seleccionado; no es una orden de sincronización con Cloud.

Parámetros como <code v-pre>{{ params.giftName }}</code> se resuelven al aplicar la plantilla. Referencias como <code v-pre>{{ event.user.uniqueId }}</code> permanecen para cada evento; las [variables globales](variables.md) aportan valores compartidos durante la ejecución.

## Resultado esperado y estado

Están disponibles las superficies locales de importación y exportación. La consulta de Cloud requiere capacidad del host, cuenta y servicio. Una vista previa del catálogo no demuestra que funcione el servicio externo de una acción. Si faltan plugins, las definiciones pueden quedar inutilizables hasta instalarlos y configurarlos.

## Recuperación

Si el JSON es inválido, sigue los errores del analizador y comprueba el tipo de documento: renombrar cualquier JSON no lo convierte. Ante conflictos de revisión, actualiza la biblioteca antes de repetir. Conserva el perfil exportado conocido y consulta [copias de seguridad](../preferences/backups.md) para los datos que la exportación no incluye.
