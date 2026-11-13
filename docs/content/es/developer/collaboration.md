---
title: "Autoridad y autenticación en colaboración"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1f211ff1f2d31b5ce901e039bb3df3c8e14a39ea680c9a5f9a840c7ac67435d9"
translationRevision: "1f211ff1f2d31b5ce901e039bb3df3c8e14a39ea680c9a5f9a840c7ac67435d9"
order: 190
---

# Autoridad y autenticación en colaboración

## Objetivo y requisitos

Permite que un dispositivo aprobado inspeccione o realice una operación acotada en un host existente. Usa un perfil de prueba del propietario, una compilación con soporte remoto y transporte configurado. Acceso a cuenta, equipo Cloud, propiedad local y sesión delegada directa son autoridades distintas.

## Establecer la cadena de confianza

La administración local `collaboration.*` controla configuración, invitaciones, solicitudes, identidades, permisos, sesiones y auditoría. Descubre sus esquemas antes de configurar escucha o crear credenciales:

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

Una invitación inicia el emparejamiento, sin conceder autoridad de negocio. La aprobación crea sesión y permisos correspondientes. HTTPS directo exige certificados válidos o confianza nativa fijada explícitamente. Tokens nativos y cookies de navegador tienen límites diferentes. `collaboration.hello` vincula protocolo, instancia, identidad, sesión, revisión de permisos, capacidades y límites negociados.

## Autorizar cada operación

Cada método remoto admitido debe superar lista permitida, capacidad, restricciones de recursos y comprobaciones de sesión actual. Deshabilitar/borrar workflows utiliza metadatos duraderos con comparación de revisión. Los ajustes de puntos son aditivos y tienen límites propios. Las lecturas remotas de automatización son resúmenes cerrados; guardar borradores desactivados de su autor no les concede autoridad ejecutable del propietario.

Revisión/adopción y aprobación comprobada vinculan definiciones, dependencias y procedencia actuales. Un manifiesto de vista previa sirve para inspección, no como token de ejecución. Revalida permisos y recursos en los límites de ejecución; la identidad del transporte no es autorización permanente.

## Resultado esperado y estado

La colaboración directa nativa implementa lecturas acotadas, mutaciones duraderas seleccionadas y recuperación limitada. Equipos/relay Cloud tienen adaptadores propios; su recorrido de lectura documentado no debe presentarse como moderación o ejecución completa de borradores. Efectos delegados restantes, otras escrituras y aceptación entre varios equipos siguen experimentales. Consulta [clientes remotos](remote-client.md) y [Cloud](cloud.md).

## Recuperación y verificación

Ante revocación o cambio de identidad, detén suscripciones y elimina proyecciones privadas. Usa `collaboration.subscribe`, `snapshot` y `resume` según dominios negociados y cursores vigentes. Resuelve el identificador original ante una confirmación incierta. No repitas un borrado con otro identificador ni restaures autoridad desde un recibo antiguo. El [código de autorización](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src) y la política del registro definen el límite; superar TLS no prueba autorización.
