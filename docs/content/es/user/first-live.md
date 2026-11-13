---
title: "Primera conexión LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "3d6c85904c001b4b8187be7d6593c55514dd3cf068237ce640bdbc7f67549e2d"
order: 30
---

# Primera conexión LIVE {#first-live-connection}

## Objetivo y requisitos {#goal-and-prerequisites}

El objetivo es recibir los primeros eventos de un creador y reconocer una conexión correcta antes de activar automatizaciones. Puedes practicar primero con los ejemplos de esta página y después repetir los pasos en la aplicación instalada.

<DocCallout title="Antes de conectar" tone="info">

Para una conexión real necesitas TikTools funcionando, acceso a internet y un creador que esté transmitiendo. Iniciar sesión en Cloud es una conexión distinta y no autentica TikTok. Los ejemplos utilizan datos ficticios y no necesitan una sesión de TikTok.

</DocCallout>

## Conectar {#connect}

<DocSteps>

1. **Abre Conexiones.** También puedes utilizar **Conectar** en la barra inferior.
2. **Introduce el usuario del creador.** El `@` inicial es opcional. Usa su identificador, no su nombre visible. En el ejemplo puedes escribir `demo_creator`.
3. **Empieza con las opciones desactivadas.** Deja **Usar una cookie de sesión** apagado si puedes entrar como invitado. Si la sala requiere una sesión autorizada, activa el control e introduce la cookie solo en su campo enmascarado.
4. **Conecta directamente.** Mantén **Vigilar y conectar automáticamente** desactivado durante esta primera prueba. Pulsa **Conectar** y espera a que se estabilice el estado.

</DocSteps>

<DemoFrame scenario="connection" locale="es" panel="connection" title="Practica tu primera conexión LIVE" />

## Resultado esperado {#expected-result}

Una conexión correcta pasa al estado **En vivo**. Para comprobar la recepción, abre **Feed en vivo**, espera un evento real o pide un mensaje de chat inocuo y verifica su creador y hora. Selecciona un evento para ver sus detalles; el nombre o avatar de un espectador abre su perfil registrado.

<DemoFrame scenario="feed" locale="es" panel="feed" title="Reconoce los primeros eventos en el feed" />

<DocCallout title="Conexión y actividad son comprobaciones distintas" tone="tip">

Una sala conectada pero sin actividad puede no producir eventos nuevos. Los puntos dependen de las reglas habilitadas; los resultados de automatización, de las acciones y eventos configurados. La demo permite reconocer estos resultados con eventos ficticios, pero no valida una sesión real de TikTok.

</DocCallout>

## Monitoreo y reconexión {#monitoring-and-reconnecting}

Tras comprobar la conexión directa, activa **Vigilar y conectar automáticamente** si quieres esperar a que el creador inicie su LIVE. El monitor distingue espera, comprobación y conexión. Detén el monitor para editar un creador bloqueado. Reconectar al mismo creador conserva el feed actual; cambiar de creador inicia otro contexto.

## Recuperación {#recovery}

Si el creador está desconectado, confirma que transmite y revisa el identificador antes de reintentar. Ante errores de autenticación o límites del servicio, abre los diagnósticos y sigue la condición indicada; repetir clics no corrige una sesión caducada.

<DocCallout title="La cookie permanece privada" tone="warning">

Si necesitas una cookie, obtén una sesión autorizada reciente. No incluyas su valor en capturas, registros ni solicitudes de soporte. Permanece en memoria y no es una credencial reutilizable de Cloud. Desconecta antes de probar otro creador.

</DocCallout>
