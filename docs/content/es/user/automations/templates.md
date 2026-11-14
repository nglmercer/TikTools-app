---
title: "Importar plantillas y administrar perfiles"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e9847c93a8a75fd5a58cd115b49b8e0bcb091ceda1dbdf2c226b52667eb17e23"
translationRevision: "e8c07fdcb5d0d3eb2b22d6577bbad8949d3af80d713151e95baf00bfe5df158c"
order: 50
---

# Importar plantillas y administrar perfiles {#import-templates-and-manage-profiles}

## Objetivo y requisitos {#goal-and-prerequisites}

Reutiliza una regla revisada o un conjunto de reglas sin activar efectos accidentalmente. Necesitas **Automations**, un archivo JSON o elemento de catálogo confiable y los plugins exigidos por sus acciones. Exporta el perfil actual antes de sustituir una configuración estable.

## Elegir la superficie adecuada {#choose-the-right-surface}

Un documento `.tiktemplate.json` describe un evento y sus acciones. Un `.tikprofile.json` agrupa varias plantillas. Abre el diálogo de plantillas desde Automations para consultar plantillas integradas o propias, su biblioteca Cloud o la importación manual. La barra de perfiles permite crear, seleccionar, exportar e importar perfiles manualmente. **Plugins → Explore** busca plugins; las plantillas y perfiles se administran aquí.

## Revisar e importar {#review-and-import}

1. Abre el diálogo correspondiente y elige un archivo o pega JSON compatible. Comprueba el nombre preparado y su origen antes de continuar.
2. Para una plantilla, inspecciona disparador, tipos de acción y parámetros. Para un perfil, revisa también el número de entradas y los parámetros comunes.
3. Sustituye destinos de ejemplo, rutas de medios e identificadores por valores adecuados a tu instalación. No pegues credenciales en archivos que vayas a compartir.
4. Aplica/importa explícitamente y espera el resultado. No supongas que las definiciones quedan desactivadas: revisa sus valores enabled. Selecciónalas después y revisa cada acción y evento antes de activar explícitamente los deseados.
5. Comienza con una prueba que solo registre texto o inspecciona primero las definiciones. Las pruebas de HTTP, audio, puntos y plugins pueden ejecutar efectos.
6. Antes de confirmar perfil, revisa que se habilitan miembros destino y deshabilitan los demás, incluidos registros sueltos y miembros antes desactivados. Los efectos requieren disparadores/condiciones; no es sync Cloud.

Parámetros como <code v-pre>{{ params.giftName }}</code> se resuelven al aplicar la plantilla. Referencias como <code v-pre>{{ event.user.uniqueId }}</code> permanecen para cada evento; las [variables globales](variables.md) aportan valores compartidos durante la ejecución.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles las superficies locales de importación y exportación. La consulta de Cloud requiere capacidad del host, cuenta y servicio. Una vista previa del catálogo no demuestra que funcione el servicio externo de una acción. Si faltan plugins, las definiciones pueden quedar inutilizables hasta instalarlos y configurarlos.

## Recuperación {#recovery}

Si el JSON es inválido, sigue los errores del analizador y comprueba el tipo de documento: renombrar cualquier JSON no lo convierte. Ante conflictos de revisión, actualiza la biblioteca antes de repetir. Conserva el perfil exportado conocido y consulta [copias de seguridad](../preferences/backups.md) para los datos que la exportación no incluye.

## Detalles de operación {#operational-details}

Importar plantilla manual primero guarda un documento de catálogo; aplicarla crea acciones/eventos enlazados. El parser usa true si falta `enabled` y la aplicación conserva esos valores. Importar perfil manual también prepara sus entradas para revisión del propietario. El catálogo Cloud guarda plantillas; un perfil Cloud prepara mutación con procedencia. Ni el origen ni una vista previa correcta garantizan desactivación universal. Revisa comparación final y estados guardados. En CLI usa expresamente `tiktools template import file.tiktemplate.json --disabled` o `tiktools template profile-import file.tikprofile.json --disabled`.

Cambiar perfil modifica activación: confirmar habilita miembros destino y deshabilita todas las acciones/eventos ajenos, incluidos registros sueltos. Miembros destino previamente desactivados pueden activarse. Revisa ambas listas antes de confirmar. Los efectos requieren disparadores, condiciones y runtime; el cambio no envía sincronización Cloud por sí mismo.
