---
title: "Administrar conexiones LIVE"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "446118d4c202b645dd93867f1715484cbd9876f940ba229ac26e4b7f24bc4cbc"
translationRevision: "78856793407bc9b944189e635b13628c244375f460d97f685fa20cd102ed28d3"
order: 10
---

# Administrar conexiones LIVE {#manage-live-connections}

## Objetivo y requisitos {#goal-and-prerequisites}

Conecta con un creador concreto, comprende la reconexión y configura los servidores de plugins por separado. Necesitas una [instalación de escritorio](../installation.md), acceso a Internet y el identificador del creador. Consulta los [requisitos](../requirements.md) antes de investigar problemas del sistema operativo.

## Conectar con intención {#connect-deliberately}

1. Abre **Connections**. Introduce el identificador, con o sin `@`; el nombre visible no es el mismo dato.
2. Comienza con **Use Cookie** desactivado. Si TikTok exige una sesión autorizada, actívalo e introduce la cookie únicamente en el campo oculto. Las cookies permanecen en memoria; no las pegues en diagnósticos ni en ajustes de Cloud.
3. Deja **Monitor and auto-connect** desactivado para un intento directo y pulsa **Connect**. Lee el estado y cualquier error antes de repetirlo.
4. Comprueba en el [feed LIVE](feed.md) que aparecen el creador correcto y un evento nuevo. Una sala conectada pero tranquila puede no mostrar actividad.
5. Usa **Reconnect** para el mismo creador o **Disconnect** antes de cambiarlo. Durante un intento en curso, **Cancel** permite detenerlo.

Tras una conexión directa satisfactoria, activa la supervisión para esperar a que el creador inicie un LIVE. Esperar y comprobar son estados distintos de estar conectado. Detén la supervisión antes de editar un identificador bloqueado. Seleccionar un creador reciente inicia la conexión; no se limita a rellenar el campo.

El explorador ofrece salas descubiertas, pero sus resultados pueden caducar. Actualízalos antes de asumir que siguen disponibles. Los servidores de plugins del panel inferior configuran servicios propios; conectar un plugin no autentica TikTok ni Cloud.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles las conexiones directas, la supervisión y los controles de plugins. Reconectar al mismo creador conserva el contexto del feed; cambiarlo inicia otro. El acceso real a TikTok depende de la sala y la sesión externas; una prueba local satisfactoria no lo garantiza.

## Recuperación {#recovery}

Si la sala está desconectada, confirma que el creador está en directo. Ante errores de sesión o límites de solicitudes, respeta la condición de reintento indicada. Reconectar repetidamente no renueva la autenticación. Consulta [solución de problemas](../preferences/troubleshooting.md) para registrar el componente y la hora sin revelar credenciales.

Pruébalo interactivamente antes de tocar tu configuración real:

<DemoFrame scenario="connection" locale="es" panel="connection" title="Prueba el panel de conexión LIVE" />

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../../.generated/cli-domain-live.es.md-->

Consulta la [referencia completa de comandos](../../developer/cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.
