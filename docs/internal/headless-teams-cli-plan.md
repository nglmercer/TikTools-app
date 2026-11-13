# Plan: Teams headless para agentes y E2E con binarios compilados

Estado: propuesta auditada; los comandos marcados como propuestos no están implementados.
Fecha: 2026-11-13. Base: `remake`, release `v4.2.2`.

## Objetivo

Permitir a un agente crear y servir un equipo, incorporarse desde otro proceso,
ejecutar operaciones autorizadas y diagnosticar errores mediante CLI/JSON,
sin necesitar la ventana de escritorio. Ejecutar el mismo flujo con binarios
de desarrollo y de release; conservar las APIs y permisos reales.

## Qué existe

- `tiktools host --ipc` y `host --stdio`: runtime headless persistente.
- `api discover`, `api schema`, `api call`, `api events` y `api verify`.
- APIs `teams.*` para crear/listar equipos, registrar dispositivos, buscar
  destinatarios por nickname, invitar, aprobar solicitudes y revocar acceso.
- `remote cloud login|enroll|join|connect|pair` para el controller nativo.
- `remote cloud pair --kind agent --profile NAME --invite-stdin`.
- `remote check NAME` y `--remote-profile NAME api call METHOD PARAMS`.
- SDK `CloudPeerController::teams()` y cliente tipado `TikToolsClient`.
- Perfiles aislables con `TIKTOOLS_HOME` y `TIKTOOLS_REMOTE_PROFILES_DIR`.
- Supervisor P2P en hosts compilados con `remote-control` y `cloud-auth`.

## Brechas comprobadas

1. `Packager.toml` y `scripts/package-release.ts` distribuyen el escritorio,
   no el ejecutable `tiktools`. La AppImage inspeccionada contiene únicamente
   `tiktools-desktop` en `usr/bin`.
2. El CLI tiene `cloud-auth,remote-client` por defecto; `remote-control` es
   opcional. El supervisor del host se compila bajo esa feature. No se puede
   prometer un host P2P completo con cualquier build del CLI.
3. Falta una familia de comandos de administración `teams`: actualmente
   el propietario tiene que componer llamadas RPC y JSON manualmente.
4. `cloud` administra la cuenta del host; `remote cloud` utiliza la cuenta
   del controller en otro directorio. Sus claves/dispositivos también son
   distintos y no deben mezclarse implícitamente.
5. `remote cloud status` consulta un manager recién construido sin restaurar
   su sesión. Debe distinguir credencial guardada, sesión comprobada y login
   pendiente, sin activar un login interactivo inesperado.
6. `remote cloud pair` tiene un límite fijo de 600 segundos e intervalos de
   10 segundos. Reintenta también errores `unauthorized`; faltan controles
   de espera y resultados intermedios estructurados para agentes.
7. El SDK puede listar membresías del controller, pero el CLI de onboarding
   no expone esa operación. Los IDs siguen siendo necesarios internamente;
   el agente necesita poder descubrirlos en JSON.
8. `--remote-profile ... rpc --stdio` está expresamente rechazado: ese modo
   inicia un host local. Falta un puente NDJSON persistente como cliente remoto.
9. La suite principal Playwright utiliza un host simulado y Vite dev. Existen
   pruebas de integración adicionales, pero falta un runner que acepte el
   binario ya compilado y recorra Teams íntegramente por CLI.

## Bloque 1: binario y comandos de propietario

Publicar un CLI portable del mismo SHA que el escritorio, con versión,
hashes y features verificables. Para servir Teams, compilar inicialmente con:

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
```

Conservar `host --ipc` para el servidor persistente. Evitar `--standalone`
para un host que deba seguir conectado después de terminar un comando.
Reutilizar el supervisor y el shutdown existentes.

Comandos propuestos, implementados como adaptadores del SDK/API existente:

| Comando propuesto | Implementación existente que reutiliza |
| --- | --- |
| `teams list --owned` | `teams.hosts.list` |
| `teams list --joined` | `CloudPeerController::teams()` en el contexto controller |
| `teams create --name NAME` | `teams.hosts.create` |
| `teams host enroll --team ID --label NAME` | `teams.devices.enroll`, kind host |
| `teams host start --wait --timeout 30` | `teams.host.start` + consulta de estado |
| `teams host status` / `teams host stop` | `teams.host.status/stop` |
| `teams people search NICKNAME` | `teams.people.search` |
| `teams invitations create --team ID --recipient ID --role ROLE` | `teams.invitations.create` |
| `teams requests list --team ID` | `teams.requests.list` |
| `teams requests approve --team ID --request ID` | `teams.requests.decide` |
| `teams grants list/update/revoke` | APIs de grants y revisión esperada existentes |
| `teams sessions list/revoke` | APIs de sesiones existentes |

La creación de invitaciones debe permitir un archivo privado creado con
permisos 0600 o un descriptor heredado; stdout JSON devuelve sólo metadatos.
La incorporación lee el token por stdin como ya hace el controller. No añadir
tokens a argumentos, historial o logs. Nicknames ambiguos requieren selección
explícita; no elegir al primer destinatario silenciosamente.

## Bloque 2: incorporación y operación del agente

Extender `remote cloud pair/connect`, manteniendo los nombres y perfiles actuales:

- `--timeout`, `--wait` y `--non-interactive`, con plazos globales acotados.
- Resultado JSON para solicitud creada, aprobación pendiente, denegación,
  confirmación del host, conexión lista y credencial inválida.
- En modo no interactivo, devolver `authentication_required` con el siguiente
  paso en vez de iniciar un navegador o esperar diez minutos silenciosamente.
- Documentar los contextos host/controller y ofrecer selección explícita.
  Compartir helpers de autenticación sin fusionar identidades de dispositivos.
- Separar errores terminales de los que admiten reintento. Un timeout después
  de una escritura deja resultado incierto; no recrear recursos a ciegas.
- Incorporar `remote cloud teams` usando el método de membresías existente.
- Añadir un cliente persistente propuesto `remote serve --profile NAME --stdio`:
  NDJSON con IDs, errores y eventos tipados, limpieza de pendientes y cierre
  por EOF/señal. Nunca iniciar AppCore ni recurrir al host local como fallback.

La primera autenticación productiva conserva el flujo real autorizado por el
usuario. Para E2E automático se usan cuentas sintéticas del backend local;
no añadir un bypass de login de pruebas al servidor productivo.

## Bloque 3: diagnóstico y APIs adicionales mínimas

No hace falta duplicar CRUD de Teams en otro backend. Agregar primero
`teams doctor` como agregador CLI de lecturas existentes: origen/cuenta,
contexto, features, dispositivo, binding del host, membresía, revisión del
grant, estado de confirmación y transporte observado.

Ampliar contratos sólo cuando una observación necesaria no esté disponible:

- Features compiladas y capacidades efectivas del host/cliente.
- Motivos estructurados de `host_not_enrolled`, `approval_pending`,
  `pending_host_confirmation`, `transport_unavailable` y `account_changed`.
- Estado del transporte y su último error sin exponer credenciales o tickets.
- Propagar `code/data` a través del SDK/CLI cuando actualmente se pierde contexto.

Mantener una sola fuente de schemas y métodos. `api discover/schema` debe
describir las ampliaciones; los comandos de conveniencia consumen ese contrato.

## Bloque 4: E2E headless sobre artefactos

Crear un runner propuesto:

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools
```

El runner provisiona un backend real en loopback y una base desechable,
reutilizando los devtools y fixtures del repositorio RemakeBackend. Inicia
un host y un controller en procesos diferentes, con homes, puertos, cuentas
y claves independientes. Acepta un binario existente; no recompila en cada caso.

Recorrido obligatorio:

1. Verificar SHA, versión, features y registro RPC del artefacto.
2. Autenticar cuentas sintéticas mediante el flujo nativo real.
3. Crear equipo, registrar host, iniciar y esperar readiness real.
4. Buscar nickname, crear invitación e incorporarse como agente.
5. Verificar que la solicitud pendiente no permite ejecutar operaciones.
6. Aprobar y esperar confirmación del host y perfil utilizable.
7. Ejecutar una lectura y una modificación permitida sobre datos ficticios;
   comprobar el resultado también desde el host propietario.
8. Rechazar una operación sin permiso y un intento desde otra cuenta.
9. Cambiar/revocar grant y sesión; comprobar que dejan de autorizar al agente.
10. Reiniciar host/controller y comprobar persistencia, recuperación, caducidad
    de invitación y reconexión. Acotar colas, retries y tiempos de espera.
11. Cerrar procesos por EOF/señal y verificar que no quedan sockets, procesos
    ni credenciales de pruebas. Conservar informes JSON/JUnit sanitizados.

Añadir casos de denegación, host offline, nickname ambiguo, carrera de revisión,
timeout de resultado incierto y cancelación durante aprobación. Ejecutar
secuencialmente los casos que compartan host; no compartir perfiles entre suites.

Un test local con dos procesos verifica cliente, backend y autorización reales;
no demuestra conectividad NAT entre dos PCs físicos. La AppImage conserva su
smoke nativo propio; las regresiones visuales siguen en las suites de UI.

## Entrega y ownership

1. CLI/release: `crates/tiktools-cli`, scripts de empaquetado y manifest de build.
2. SDK/contratos: `crates/tiktools-client` y `tiktools-control-api` sólo si falta
   una observación; conservar lógica de negocio en sus servicios actuales.
3. Backend: devtools/fixtures para procesos externos en RemakeBackend; no
   modificar los cambios ajenos actuales ni crear endpoints de prueba públicos.
4. Runner: helpers y suite nuevos, registro en `scripts/task.ts`, guía de uso
   para agentes basada en `--help`, JSON y ejemplos realmente ejecutados.

Prioridad: binario headless con features correctas → administración por CLI →
onboarding/diagnóstico acotados → E2E con artefacto → puente persistente de agentes.
Ejecutar localmente y por filtros; no lanzar Actions ni releases para este plan.

## Evidencia de esta revisión

La AppImage 4.2.2 pasó la prueba existente de arranque montado/FUSE con perfil
aislado: versión correcta, tipo `appImage`, `WebView frontend-ready`.
Log local: `/tmp/tiktools-422-plan-native-smoke.log`. No es todavía el E2E Teams
propuesto y no se efectuaron escrituras en la cuenta personal.

Fuentes auditadas: `crates/tiktools-cli/src/remote/cloud.rs`, `host.rs`,
`main.rs`, `commands/api.rs`, `crates/tiktools-client/src/remote/cloud_peer.rs`,
`crates/tiktools-control-api/src/modules/teams.rs`,
`crates/tiktools-core/src/control/cloud_teams.rs`, `Packager.toml`,
`scripts/package-release.ts`, `playwright.config.ts` y las fixtures SDK del backend.
