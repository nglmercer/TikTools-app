| Comando | Resumen |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | listar todos los métodos en vivo del registro del host |
| `tiktools api schema <method> [--format json&#124;human]` | descripción, flags y esquemas de un método |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validar y ejecutar un método; los destructivos necesitan --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | transmitir eventos de dominio NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | buscar en el diario acotado del feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verificar la superficie de control contra el host |
