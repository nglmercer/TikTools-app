| Command | Summary |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | list automation rules |
| `tiktools automation context [--event-type t]` | template context available to rules |
| `tiktools automation get <id> [--kind event&#124;action]` | read one rule |
| `tiktools automation create --record json [--kind event&#124;action]` | create a rule |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | update a rule |
| `tiktools automation delete <id> [--kind event&#124;action]` | delete a rule **destructive** |
| `tiktools automation enable <id> [--kind event&#124;action]` | enable a rule |
| `tiktools automation disable <id> [--kind event&#124;action]` | disable a rule |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | dry-run a rule against a sample event |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | fire a trigger manually |
| `tiktools automation moderation-penalty --points <delta>` | apply a moderation penalty in points |
