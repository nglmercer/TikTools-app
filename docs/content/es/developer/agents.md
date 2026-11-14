---
title: "Agentes y SDK tipado de Rust"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
translationRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
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

## Teams headless para agentes {#headless-teams}

Compila el CLI con soporte de host o usa el CLI de la misma versión incluido en la release portable. Ejecuta el propietario como proceso persistente con un `TIKTOOLS_HOME` absoluto y aislado. Los comandos de otra terminal deben usar ese mismo directorio.

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` autentica el runtime propietario. `remote cloud login` autentica el contexto controller, cuyos metadatos de cuenta y claves de dispositivo se mantienen separados. Ambos usan el handoff real del navegador; no requieren la ventana de escritorio. `--non-interactive` nunca inicia un login y devuelve `authentication_required` si faltan credenciales.

El propietario crea el equipo, registra su dispositivo host e inicia el servicio. Usa `teams people search NICKNAME` para descubrir el ID del destinatario y crear una invitación vinculada a él. El token se escribe una vez en un archivo privado nuevo; stdout contiene sólo metadatos. Aprueba la solicitud explícitamente y configura su grant nativo de agente antes de conectar.

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

Para acceso reutilizable, usa `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` después de aprobar. `teams list --joined` consulta membresías del controller; `teams list --owned` consulta equipos del propietario. `teams doctor` inspecciona el propietario; `teams doctor --context controller` inspecciona la autenticación y membresías del controller. `remote check agent` verifica una conexión real.

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

El cliente persistente admite una petición JSON-RPC por línea y conserva los IDs. Las escrituras requieren `confirm: true`; las escrituras duraderas necesitan también sus metadatos documentados de comando y host. EOF, una señal de terminación o el cierre de conexión liberan el cliente. Las peticiones tienen tamaños y tiempos acotados; una escritura con timeout puede haberse confirmado.

El runner de aceptación usa un CLI compilado, un backend real desechable, cuentas sintéticas y procesos separados para propietario y controller. Indica un checkout de RemakeBackend con la fixture de aceptación CLI y sus herramientas pgrust instaladas. Los binarios de desarrollo aceptan el origen aislado en runtime. Las releases conservan el origen fijado al compilar: compila una release de staging para un puerto TLS fijo de loopback y pasa `--cloud-port PORT`. Los binarios fijados a producción se rechazan antes de autenticar. No se inicia ningún workflow remoto.

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

Los nuevos paquetes también admiten `./TikTools.AppImage --cli --build-info` y `./TikTools.AppImage --cli --json teams doctor`. Esta entrada despacha antes de inicializar la GUI; las AppImages publicadas anteriormente no la incorporan automáticamente.

La reutilización de un perfil controller entre procesos del CLI requiere un almacén de credenciales del sistema operativo disponible.
