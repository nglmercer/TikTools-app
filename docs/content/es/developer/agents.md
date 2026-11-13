---
title: "Agentes y SDK tipado de Rust"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
translationRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
order: 80
---

# Agentes y SDK tipado de Rust {#agents-and-the-typed-rust-sdk}

## Objetivo y requisitos {#goal-and-prerequisites}

Construye scripts o agentes contra el mismo host que utiliza el escritorio. Necesitas un host aislado activo, la CLI `tiktools` correspondiente y acceso al [descubrimiento](control-api.md). El SDK está en `crates/tiktools-client`; ofrece acceso directo, IPC local y transporte remoto opcional sin crear otro núcleo de negocio en las rutas de cliente normales.

## Comenzar con descubrimiento de lectura {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

Resuelve capacidades y esquemas antes de generar una acción. `system.info` no demuestra conexión TikTok, disponibilidad OBS ni permiso de mutación. `system.doctor` distingue comprobaciones reales y omitidas de las capacidades de compilación.

Una función asíncrona Rust puede conectarse al propietario existente:

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

El llamante aporta su runtime Tokio. Los métodos tipados delegan en RPC registrados y la prueba de paridad protege su cobertura. Utiliza `call_with_meta` o `call_value_with_meta` cuando workflows, automatizaciones o diarios del propietario necesiten versiones y recibos; los métodos de dominio mantienen deliberadamente resultados más sencillos.

## Enviar cambios intencionales {#submit-intentional-changes}

Para una operación duradera, conserva método, parámetros canónicos, identificador original, instancia del host y revisión exigida por descubrimiento. Guarda el candidato del usuario antes de enviarlo. Un borrador remoto de automatización debe permanecer explícitamente deshabilitado y pertenecer a su autor; queda pendiente de revisión, sin autorización de ejecución. La adopción y activación del propietario tienen contratos locales separados.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

Un agente recibe las mismas validaciones, eventos y errores que otros clientes. El acceso de agentes está implementado; la autoridad depende del backend seleccionado y de los permisos actuales. Una confirmación incierta exige consultar el resultado original, no repetir automáticamente con otro identificador. Los resúmenes remotos cerrados no reconstruyen definiciones completas: conserva privadamente el código enviado.

Ejecuta `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` al añadir métodos. El [código del SDK](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) y el registro canónico definen las firmas; los ejemplos antiguos pueden preceder a los nuevos coordinadores de recuperación del propietario.
