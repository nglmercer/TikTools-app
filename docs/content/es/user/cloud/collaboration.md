---
title: "Preparar colaboración y acceso de equipos"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "es"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "2fbcef6dfd6d359e74bc4db46dcadc42e42a4b172f42bc0b10c7ec9475ff747b"
order: 30
---

# Preparar colaboración y acceso de equipos {#prepare-collaboration-and-team-access}

## Objetivo y requisitos {#goal-and-prerequisites}

Asigna a otro operador un papel definido manteniendo el control del propietario. El acceso remoto lo ofrece el plugin instalable **Remote Control**, no una pestaña de ajustes: necesitas [sesión de Cloud](account.md), un backend compatible y el plugin instalado en este host. Comienza con acceso de lectura en un entorno de prueba.

## Configurar el acceso remoto {#configure-remote-access}

1. Instala y habilita `tiktools.remote-control` y abre su página **Remote control**.
2. Crea un equipo y registra **este PC** como host con una etiqueta de dispositivo reconocible; inicia el enlace de retransmisión y espera a que su estado sea `ready`.
3. Crea una invitación y entrega su token de un solo uso por un canal confiable. Una invitación no es una sesión activa aprobada.
4. Revisa la solicitud pendiente y el papel solicitado. Aprueba solo al operador esperado; la concesión permanece `pendingHost` hasta que el enlace del host la confirma.
5. Revisa periódicamente las concesiones y los dispositivos. Revoca una concesión cuando deje de ser necesaria.

Editar una definición delegada, adoptarla como propietario y aprobar su ejecución son decisiones distintas. Lee el código guardado, las acciones referenciadas y los permisos en la revisión específica. Una vista previa por sí sola no autoriza efectos.

## Preparar un dispositivo controlador {#prepare-a-controller-device}

En el dispositivo controlador, inicia sesión con su propia cuenta, regístralo como controlador y canjea el token de invitación. El propietario verá entonces una solicitud pendiente. La búsqueda por apodo requiere un endpoint de backend desplegado por separado y puede no estar disponible en el servicio actual. Un fallo de búsqueda no demuestra que la persona carezca de cuenta; tampoco sustituyas la selección por un ID arbitrario.

## Resultado esperado y estado {#expected-result-and-status}

El flujo entre dos cuentas (equipo, enlace del host, invitación, aprobación, comando de lectura y revocación) se ha verificado con un backend local. Las pestañas de colaboración/equipos en ajustes fueron eliminadas; el servicio nativo de colaboración no tiene interfaz web en esta versión. El comportamiento de retransmisión/NAT en producción, la búsqueda por apodo y la operación LIVE entre dos PC no están aquí comprobados. Un enlace listo no demuestra por sí solo el funcionamiento íntegro de los permisos para tu acción.

## Recuperación {#recovery}

Para concesiones obsoletas, revoca el acceso y revísalo de nuevo tras la actualización del enlace. Ante comandos sin confirmar, inspecciona el resultado registrado antes de repetir. Detén el enlace o revoca el acceso al terminar la sesión. Mantén tokens de invitación y credenciales fuera de los [informes públicos](../preferences/troubleshooting.md).

## Detalles de operación {#operational-details}

Cada comando remoto pasa por la compuerta de capacidades del host (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`); el plugin nunca recibe una Control API ni un token completo. Un transporte activo o la pertenencia a un equipo no conceden acceso nativo por sí solos. Observer orienta a lectura; moderator añade controles concretos, automationEditor edición concreta y custom requiere una política explícita revisada. Ninguno concede scripts/efectos ilimitados. Consulta [roles y confirmación](../../developer/collaboration.md#operational-details) y [transports](../../developer/transports.md) para conocer el límite actual.
