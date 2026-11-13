---
title: "Administrar conexiones LIVE"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "37f7274550d3010051ab456da2a2091b650e5e7aaaf30756b6695435bcfc08c7"
translationRevision: "37f7274550d3010051ab456da2a2091b650e5e7aaaf30756b6695435bcfc08c7"
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
