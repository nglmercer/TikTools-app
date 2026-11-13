---
title: "Primera conexión LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb0f6071475eae033404ecb741bd0e84e265acb737d13de14d96dae78f02e729"
translationRevision: "fb0f6071475eae033404ecb741bd0e84e265acb737d13de14d96dae78f02e729"
order: 30
---

# Primera conexión LIVE

## Objetivo y requisitos

Recibir eventos de un creador antes de activar automatizaciones. Necesitas una instalación funcional, acceso a internet y un creador que esté transmitiendo. Iniciar sesión en Cloud es una conexión distinta y no autentica TikTok.

## Conectar

1. Abre **Connections** o la acción **Connect** de la barra inferior.
2. Escribe el handle del creador. El `@` inicial es opcional. Comprueba el identificador, no el nombre visible.
3. Empieza con **Use Cookie** desactivado cuando sea posible acceder como invitado. Si la sala requiere una sesión autorizada, activa el control e introduce la cookie sólo en su campo enmascarado.
4. Deja el monitoreo apagado durante esta primera conexión directa. Pulsa **Connect** y espera a que se estabilice el estado.
5. Abre **Live feed**. Espera un evento real o pide un mensaje de chat inocuo. Comprueba su creador y hora.
6. Selecciona un evento para ver sus detalles. Selecciona el nombre o avatar de un espectador para consultar su perfil registrado.

## Resultado esperado

El estado pasa a conectado y aparecen los eventos entrantes. Una sala sin actividad puede no producir eventos nuevos. Los puntos dependen de las reglas habilitadas y los resultados de automatización dependen de las acciones y eventos configurados.

## Monitoreo y reconexión

Tras comprobar una conexión directa, **Monitor and auto-connect** puede esperar a que el creador inicie su LIVE. Su estado diferencia espera, comprobación y conexión. Detén el monitoreo para editar un creador bloqueado. Reconectar al mismo creador conserva el feed actual; cambiar de creador inicia otro contexto.

## Recuperación

Si el creador está desconectado, confirma que transmite antes de reintentar. Ante errores de autenticación o límites del servicio, abre **Diagnostics** y sigue la condición indicada; repetir clics no corrige una sesión caducada. Si necesitas cookie, obtén una sesión autorizada reciente y no incluyas su valor en capturas, registros o solicitudes de soporte. Permanece en memoria y no es una credencial reutilizable de Cloud.

Desconecta antes de probar otro creador. La demo permite aprender estos controles con eventos ficticios, pero su resultado no valida una sesión de TikTok.

<DemoFrame scenario="connection" locale="es" tour="connection" title="Demo de connection" />
