---
title: "Respaldar y restaurar el trabajo local"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
translationRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
order: 20
---

# Respaldar y restaurar el trabajo local {#back-up-and-restore-local-work}

## Objetivo y requisitos {#goal-and-prerequisites}

Conserva una configuración funcional antes de actualizar, reiniciar datos o cambiar de equipo. Necesitas almacenamiento bajo tu control y tiempo para cerrar TikTools correctamente. La interfaz actual no incluye un botón universal para respaldarlo todo; combina exportaciones específicas con una copia de datos hecha con la aplicación cerrada.

## Exportar definiciones reutilizables {#export-reusable-definitions}

1. En **Automations**, selecciona el perfil deseado y expórtalo a un archivo `.tikprofile.json`. Incluye versión y fecha en su nombre.
2. Para una plantilla individual, usa la descarga/exportación del diálogo cuando esté disponible y guarda `.tiktemplate.json`.
3. Inspecciona el archivo antes de compartirlo. Las plantillas pueden contener destinos y valores de configuración. Exportar un perfil no copia saldos, estadísticas, variables de ejecución, medios ni credenciales de cuenta.

## Copiar los datos locales {#copy-the-local-data}

1. Detén la actividad LIVE y el servicio de colaboración y sal de TikTools normalmente. Espera a que el host termine de escribir.
2. Localiza la raíz activa. Los valores predeterminados son `%LOCALAPPDATA%\TikTools` en Windows y `~/.local/share/TikTools` en Linux, o `TikTools` dentro de `XDG_DATA_HOME` si está configurado. `TIKTOOLS_HOME` y las rutas particulares pueden cambiar estas ubicaciones.
3. Copia la raíz y los directorios de datos, plugins, datos de plugins y almacén de plugins que se hayan configurado por separado a una carpeta fechada. Copia el directorio completo de bases cerradas, incluidos los archivos auxiliares SQLite presentes, en lugar de seleccionar solo un `.db`.
4. Conserva aparte los archivos de audio referenciados: las acciones guardan rutas, no los bytes del sonido. Anota versión y ubicaciones personalizadas junto a la copia.
5. Protege el respaldo porque la configuración puede incluir valores privados. La autenticación de Cloud permanece en el almacén del sistema operativo y no se transporta mediante esta copia.

## Restaurar y verificar {#restore-and-verify}

Con TikTools cerrado, conserva la carpeta actual como recuperación separada. Restaura los archivos a las rutas previstas e inicia una versión compatible. Inspecciona perfil, saldos, variables y medios antes de conectar. Para definiciones portátiles, importa el perfil o plantilla desde Automations; permanecen desactivadas hasta su activación explícita.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

Las copias con la aplicación cerrada y las superficies indicadas están disponibles, sin garantía universal entre versiones. Changes de Cloud no es un respaldo completo. Si la versión rechaza los datos, detente y recupera la copia preservada; reinstalar un ejecutable anterior no revierte una migración. Consulta [actualizaciones](../updates.md) y [solución de problemas](troubleshooting.md).
