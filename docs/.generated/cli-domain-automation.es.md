| Comando | Resumen |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | listar las reglas de automatización |
| `tiktools automation context [--event-type t]` | contexto de plantilla disponible para las reglas |
| `tiktools automation get <id> [--kind event&#124;action]` | leer una regla |
| `tiktools automation create --record json [--kind event&#124;action]` | crear una regla |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | actualizar una regla |
| `tiktools automation delete <id> [--kind event&#124;action]` | eliminar una regla **destructivo** |
| `tiktools automation enable <id> [--kind event&#124;action]` | activar una regla |
| `tiktools automation disable <id> [--kind event&#124;action]` | desactivar una regla |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | probar una regla en seco contra un evento de muestra |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | disparar un trigger manualmente |
| `tiktools automation moderation-penalty --points <delta>` | aplicar una penalización de moderación en puntos |
