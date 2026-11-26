| Comando | Resumen |
| --- | --- |
| `tiktools host --stdio [--events]` | host sin interfaz en stdio; transmite eventos con --events |
| `tiktools host --ipc` | host sin interfaz en IPC local; rechaza si ya hay un host en ejecución |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | paso directo sin tipos a cualquier método del host |
| `tiktools rpc --stdio [--events]` | bucle de petición/respuesta NDJSON en stdio |
