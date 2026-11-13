---
title: "Iniciar sesión en tu cuenta de Cloud"
docId: "user.cloud.account"
translationKey: "user.cloud.account"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "02b7718e7fc7b2a53b68ee1c6e19e622608a6fdfc7eac553879c474ca69ea74b"
translationRevision: "c621eebc98665ab57a08cd00eae611a2fee2925a81854f247fbfcf5042ec4b6a"
order: 10
---

# Iniciar sesión en tu cuenta de Cloud {#sign-in-to-your-cloud-account}

## Objetivo y requisitos {#goal-and-prerequisites}

Conecta la aplicación de escritorio a su servicio Cloud configurado para operaciones de catálogo y colaboración vinculadas a una cuenta. Necesitas Internet, un navegador del sistema y un almacén de credenciales del sistema operativo funcional. La edición local sigue siendo útil sin Cloud; su sesión no autentica la conexión LIVE de TikTok.

## Iniciar y verificar la identidad {#sign-in-and-verify-identity}

1. Abre **Settings → Account**. Lee el estado actual antes de iniciar otro acceso.
2. Pulsa **Sign in**. El escritorio inicia un receptor de retorno local y abre la página de acceso configurada en el navegador del sistema.
3. Completa el acceso del servicio y vuelve a TikTools. Si no se abrió el navegador, utiliza la URL mostrada para ese intento. Trátala como privada y no la publiques.
4. Espera a que el estado indique sesión iniciada. Confirma nombre o correo antes de abrir un catálogo privado o equipo.
5. Despliega los diagnósticos de Account si aparece un error. Usa **Cancel** para abandonar un intento pendiente en lugar de mantener varios activos.
6. Al terminar en un equipo compartido, pulsa **Sign out** y verifica el cambio de estado.

El host nativo administra el token de sesión. Guarda las credenciales mediante el almacén del sistema operativo y solo entrega estado no secreto a la interfaz. No existe una alternativa de archivo en texto plano si falla el almacenamiento seguro. Copiar la carpeta de datos no transporta una sesión autenticada.

## Resultado esperado y estado {#expected-result-and-status}

El acceso nativo, la restauración y el cierre de sesión están disponibles en hosts compilados con autenticación Cloud. Una cuenta configurada puede restaurarse al reiniciar y validarse de nuevo con el servicio. Introducir otra cookie de TikTok no repara un host sin Cloud o sin configuración.

La disponibilidad del acceso y la aceptación real de proveedores y plataformas dependen del servicio desplegado y del equipo. Una validación local no demuestra que funcionen todas las rutas de producción.

## Recuperación {#recovery}

Si el intento caducó o se canceló, ciérralo e inicia uno nuevo. Ante fallos de red o servicio, conserva el código de diagnóstico y repite tras recuperarse. Si falla el almacén de credenciales, restablece el almacenamiento seguro del sistema; no pegues tokens en ajustes ni en mensajes de soporte. Consulta [Changes](changes.md) para sincronización explícita y [colaboración](collaboration.md) para los permisos separados.

## Detalles de operación {#operational-details}

Cerrar sesión elimina credenciales locales y limpia el contexto antes de intentar revocación remota de mejor esfuerzo. La UI sin sesión confirma retirada local, no revocación remota durante fallo de red. Usa administración de sesiones del servicio si existe. Exporta Diagnostics antes de cerrar/cambiar cuenta conocida: se limpia ese historial UI. Cookies TikTok y sesiones de host remoto son identidades separadas.
