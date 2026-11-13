---
title: "Clientes remotos y recuperación"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9c02df03cbaef36512591b4dd92e4713db47f3f317917dbef4191bc598bd3cdc"
translationRevision: "9c02df03cbaef36512591b4dd92e4713db47f3f317917dbef4191bc598bd3cdc"
order: 90
---

# Clientes remotos y recuperación

## Objetivo y requisitos

Conecta un controlador a un host sin crear un núcleo LIVE local. Necesitas un host con las características remotas activadas, una invitación aprobada por el propietario, un origen HTTPS accesible y un almacén seguro compatible. Lee [colaboración](collaboration.md) para los permisos y [transportes](transports.md) para las rutas nativas.

## Emparejar e inspeccionar

Estas órdenes corresponden a un entorno manual de prueba; sustituye el origen de ejemplo por el configurado:

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

Si el certificado es autofirmado, compara su huella con el propietario por otro canal y añade `--fingerprint` al emparejar. Inspeccionar no envía operaciones autenticadas ni aprueba confianza. Introduce invitaciones y credenciales mediante las preguntas admitidas, nunca como secretos en los argumentos.

Los archivos de perfil contienen identidad, política de confianza y referencia opaca al almacén seguro. Linux usa Secret Service, macOS Keychain y Windows Credential Manager. Un almacén inaccesible falla explícitamente; Android no tiene backend de perfiles persistidos. El SDK admite credenciales proporcionadas explícitamente en memoria. `remote forget` elimina los datos locales, pero no revoca la sesión del host.

## Conservar la identidad de las órdenes

Lee el recurso con metadatos antes de una mutación admitida. Conserva revisión base, instancia del host e identificador duradero. Si la conexión se corta tras enviarla, consulta `collaboration.commands.get` con ese mismo identificador. Un recibo es histórico: actualiza el recurso aparte. Deshabilitar/borrar workflows, ajustar puntos y enviar borradores desactivados tienen contratos y permisos diferentes.

## Resultado esperado y estado

El escritorio nativo ofrece `tiktools-desktop --controller` y diarios vinculados al perfil. La cobertura sigue siendo experimental. Emparejar, conectar o recibir una bandera general no autoriza cualquier edición. Las lecturas remotas de automatización muestran resúmenes cerrados, nunca código editable; activar/ejecutar remotamente y adoptar como propietario son límites distintos.

## Recuperación

Ante un hueco de secuencia, usa instantánea y reanudación negociadas; borra proyecciones privadas al cambiar la identidad. No repitas mutaciones automáticamente al reconectar. Un certificado cambiado exige revisar la confianza. La [guía de implementación](https://github.com/nglmercer/TikTools-app/blob/remake/docs/LIVE-COLLABORATION-CLIENT.md) explica exclusión de perfiles e incertidumbre de renovación; distingue pruebas locales de aceptación real con varios equipos y almacenes del sistema.
