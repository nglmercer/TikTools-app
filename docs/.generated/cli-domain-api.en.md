| Command | Summary |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | list every method live from the host registry |
| `tiktools api schema <method> [--format json&#124;human]` | one method's description, flags and schemas |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validate and execute one method; destructive methods need --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | stream NDJSON domain events |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | search the bounded feed journal |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verify the control surface against the host |
