---
title: "Preparar colaboración y acceso de equipos"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "es"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "46c72bb38652a500095b2525c5e58783ba111a3abeb7303346d06acfbaae8483"
translationRevision: "46c72bb38652a500095b2525c5e58783ba111a3abeb7303346d06acfbaae8483"
order: 30
---

# Preparar colaboración y acceso de equipos

## Objetivo y requisitos

Asigna a otro operador un papel definido manteniendo el control del propietario. Necesitas un host de escritorio compatible y elegir entre colaboración local/nativa y Teams vinculado a cuentas. Teams también requiere [sesión de Cloud](account.md) y soporte del servicio desplegado. Comienza con acceso de lectura en un entorno de prueba.

## Configurar acceso del propietario

1. Abre **Settings → Collaboration**. Inspecciona el estado y activa/configura el receptor o la pasarela previstos mediante sus controles.
2. Revisa los orígenes permitidos y la matriz de acceso antes de crear una invitación. Elige únicamente las áreas, operaciones y recursos necesarios.
3. Crea la invitación y entrega su enlace/token privado por un canal confiable. Una invitación no equivale a una sesión activa aprobada.
4. Inspecciona las solicitudes pendientes y sus permisos. Aprueba solo al operador esperado y confirma su identidad visible y conexión.
5. Usa el inspector de acceso para revisar modificaciones posteriores. Revoca invitaciones, identidades o sesiones cuando dejen de necesitar acceso.

Editar una definición delegada, adoptarla como propietario y aprobar su ejecución son decisiones distintas. Lee el código guardado, las acciones referenciadas y los permisos en la revisión específica. Una vista previa por sí sola no autoriza efectos.

## Preparar un equipo vinculado a cuentas

Cuando aparezca **Settings → Teams**, crea un equipo con nombre y selecciónalo. Registra **este PC** como host con una etiqueta reconocible, pulsa **Start serving** e inspecciona el enlace. La creación y el registro del host local están conectados en el frontend; son pasos distintos de invitar a otra persona. Los dispositivos controladores tienen su propio papel de registro.

Elige destinatario, papel y duración de invitación; revisa solicitudes, miembros y dispositivos. La búsqueda por apodo requiere un endpoint de backend desplegado por separado y puede no estar disponible en el servicio actual. Un fallo de búsqueda no demuestra que la persona carezca de cuenta; tampoco sustituyas la selección por un ID arbitrario.

## Resultado esperado y estado

Existen controles del propietario y flujos frontend de creación/registro de equipos. La colaboración remota completa es experimental: aquí no se acredita operación LIVE entre dos PC reales, búsqueda en producción, comportamiento NAT/retransmisión ni recuperación de todos los efectos y plataformas. Un enlace preparado no demuestra por sí solo el funcionamiento íntegro de los permisos para tu acción.

## Recuperación

Si cambian permisos o revisiones del host, carga y revisa otra vez. Ante órdenes sin confirmar, inspecciona el resultado registrado antes de repetir. Detén el servicio o revoca acceso al terminar. Mantén tokens de invitación y credenciales fuera de los [informes públicos](../preferences/troubleshooting.md).

<DemoFrame scenario="collaboration" locale="es" title="Demo de collaboration" />
