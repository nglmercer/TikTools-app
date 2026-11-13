---
title: "Preguntas frecuentes"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
order: 40
---

# Preguntas frecuentes {#frequently-asked-questions}

## Objetivo y requisitos {#goal-and-prerequisites}

Resuelve dudas habituales antes de modificar datos o credenciales. Necesitas la aplicación de escritorio para las comprobaciones; [requisitos](../requirements.md) describe los paquetes compatibles. Las funciones locales mencionadas están disponibles salvo que se indique una limitación experimental o dependiente de servicios.

## ¿Iniciar sesión en Cloud conecta TikTok? {#does-cloud-sign-in-connect-tiktok}

No. Abre [Connections](../live/connections.md), elige un creador e inicia LIVE por separado. La identidad Cloud sirve a operaciones vinculadas a cuentas. La cookie de TikTok pertenece únicamente a su campo oculto de conexión.

## ¿Por qué está vacío un feed conectado? {#why-is-a-connected-feed-empty}

Una sala tranquila puede no generar eventos nuevos. Selecciona todos, borra la búsqueda, reanuda el desplazamiento y comprueba la conexión. Pausar la vista mantiene la recepción y las automatizaciones. Vaciar el feed no reinicia puntos ni estadísticas persistentes.

## ¿Las estadísticas son solo de esta sesión? {#are-statistics-only-for-this-session}

La [página Analytics](../points/statistics.md) actual consulta agregados persistentes del creador y período seleccionados. Amplía las fechas y actualiza para revisar historial registrado. No es una transcripción completa. Los saldos utilizan reglas distintas y pueden incluir ajustes manuales o de automatizaciones.

## ¿Dónde están plantillas, variables y rankings? {#where-are-templates-variables-and-rankings}

Las plantillas y perfiles están en [Automations](../automations/templates.md); Explore de Plugins solicita plugins. Los valores de ejecución están en Variables de Automations. El espacio experimental de rankings conserva `#/rankings`, pero está oculto en la navegación y no se restaura al iniciar. Las tablas de puntos y paneles del feed siguen siendo vistas separadas.

## ¿Guardar sincroniza todo? {#does-saving-synchronize-everything}

No. Changes de Eventos/Acciones compara una referencia local. Variables guarda localmente y ofrece operaciones revisadas separadas de envío/adopción/retirada cuando son compatibles. Los secretos se ocultan en comparaciones. Las superposiciones Cloud usan creación/revisión y biblioteca; esta versión no tiene un adaptador Changes compartido. Lee [Changes](../cloud/changes.md) antes de elegir dirección.

## ¿Las pruebas son inocuas y las importaciones se ejecutan inmediatamente? {#are-tests-harmless-and-do-imports-run-immediately}

Las pruebas pueden producir efectos, incluidos audio, solicitudes de red y cambios de puntos. Comienza con [core.log](../automations/first-rule.md). Las definiciones importadas permanecen desactivadas: revísalas y actívalas explícitamente. Instalar un plugin también exige configurar su servicio antes de utilizar sus acciones.

## Resultado esperado y recuperación {#expected-result-and-recovery}

Estas comprobaciones deberían identificar la página responsable sin reiniciar datos. Las consultas automáticas de actualización son opcionales; la instalación es experimental y está desactivada en el flujo de publicación actual. La colaboración remota completa y la búsqueda por apodo en producción dependen del despliegue y la aceptación. Si persiste una duda, conserva [copias](backups.md) y reúne un [informe de diagnóstico](troubleshooting.md) con versión, hora y reproducción.
