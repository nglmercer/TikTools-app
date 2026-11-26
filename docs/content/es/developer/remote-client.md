---
title: "Clientes remotos y recuperación"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1f6c56c60e69e8804bf94b4b0b86311da33a0a65506c5d93501804a24abe4a1b"
translationRevision: "4ee7a75892bc896409dd9d3128dde8823a1d1d8da8e227e6dde46fad1a3b6a24"
order: 90
---

# Clientes remotos y recuperación {#remote-clients-and-recovery}

## Objetivo y requisitos {#goal-and-prerequisites}

Conecta un controlador a un host sin crear un núcleo LIVE local. Necesitas un host con las características remotas activadas, una invitación aprobada por el propietario, un origen HTTPS accesible y un almacén seguro compatible. Lee [colaboración](collaboration.md) para los permisos y [transportes](transports.md) para las rutas nativas.

## Emparejar e inspeccionar {#pair-and-inspect}

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

## Conservar la identidad de las órdenes {#preserve-command-identity}

Lee el recurso con metadatos antes de una mutación admitida. Conserva revisión base, instancia del host e identificador duradero. Si la conexión se corta tras enviarla, consulta `collaboration.commands.get` con ese mismo identificador. Un recibo es histórico: actualiza el recurso aparte. Deshabilitar/borrar workflows, ajustar puntos y enviar borradores desactivados tienen contratos y permisos diferentes.

## Resultado esperado y estado {#expected-result-and-status}

El escritorio nativo ofrece `tiktools-desktop --controller` y diarios vinculados al perfil. La cobertura sigue siendo experimental. Emparejar, conectar o recibir una bandera general no autoriza cualquier edición. Las lecturas remotas de automatización muestran resúmenes cerrados, nunca código editable; activar/ejecutar remotamente y adoptar como propietario son límites distintos.

## Recuperación {#recover}

Ante un hueco de secuencia, usa instantánea y reanudación negociadas; borra proyecciones privadas al cambiar la identidad. No repitas mutaciones automáticamente al reconectar. Un certificado cambiado exige revisar la confianza. La [implementación de perfiles](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs) explica exclusión de perfiles e incertidumbre de renovación; distingue pruebas locales de aceptación real con varios equipos y almacenes del sistema.

## Detalles de operación {#operational-details}

Aplica cada evento fiable a la instantánea/proyección correspondiente antes de `acknowledge`; confirmar antes pierde evidencia de recuperación. Reconecta/reanuda solo con la misma instancia, suscripción, sesión y permiso. Un hueco, contexto cambiado, cursor rechazado o cierre del navegador por gap requiere instantáneas autorizadas nuevas y otra suscripción. No repitas mutaciones desde el historial. Usa la [receta de metadatos/ID original](control-api.md#operational-details) para escrituras inciertas.

El `event` local contiene `{topic,data?}`; variantes unitarias pueden omitir datos. En remoto `event.params` es ReliableEvent, con secuencia de texto, ID, versión de recurso y actor. `event.gap` es otro método. Los sobres siguientes son ilustrativos, no replay durable de efectos perdidos.

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

El esquema SDK siguiente requiere remote-profiles. Los callbacks de proyección pertenecen a la aplicación y terminan antes de acknowledge. Véase [RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) para contratos de baseline, cursor y reanudación.

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../.generated/cli-domain-web.es.md-->

Consulta la [referencia completa de comandos](cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.
