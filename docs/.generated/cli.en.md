`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — install and manage plugins

| Command | Summary |
| --- | --- |
| `tiktools plugin list` | list installed plugins |
| `tiktools plugin get <id>` | read one plugin |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | install from a file, id@version or url |
| `tiktools plugin uninstall <id> [--purge]` | uninstall a plugin **destructive** |
| `tiktools plugin search <query> [--category c] [--page N]` | search packages in the registry |
| `tiktools plugin inspect <spec>` | inspect a package spec before installing |
| `tiktools plugin outdated` | list installed plugins with available updates |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | update one plugin |
| `tiktools plugin update --all [--yes] [--no-wait]` | update every outdated plugin |
| `tiktools plugin rollback <id> [version] [--no-wait]` | roll back to a previous version **destructive** |
| `tiktools plugin versions <id>` | list available versions of a plugin |
| `tiktools plugin pin <id>` | pin a plugin to its current version |
| `tiktools plugin unpin <id>` | remove the version pin |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | set channel and auto-update policy |
| `tiktools plugin jobs` | list package jobs |
| `tiktools plugin job <job-id>` | inspect one package job |
| `tiktools plugin cancel <job-id>` | cancel a running package job **destructive** |
| `tiktools plugin enable <id>` | enable a plugin |
| `tiktools plugin disable <id>` | disable a plugin |
| `tiktools plugin start <id>` | start a plugin |
| `tiktools plugin stop <id>` | stop a plugin **destructive** |
| `tiktools plugin settings get <id>` | read plugin settings |
| `tiktools plugin settings set <id> k=v...` | write plugin settings |
| `tiktools plugin settings reset <id>` | reset plugin settings **destructive** |
| `tiktools plugin health <id>` | plugin health check |
| `tiktools plugin options <action-type/field>` | options of an action type or field |
| `tiktools plugin action <type> [--config json] [--live]` | run a plugin action |
| `tiktools plugin commands` | list commands contributed by installed plugins |
| `tiktools plugin help [id]` | describe one contributed command |
| `tiktools plugin <id> <command> [--params json] [--live]` | invoke a plugin-contributed command |

### `tiktools cloud` — cloud account and portable backup/restore (override with TIKTOOLS_CLOUD_ORIGIN)

| Command | Summary |
| --- | --- |
| `tiktools cloud status` | cloud account status |
| `tiktools cloud login` | sign in through the system browser |
| `tiktools cloud logout` | sign out **destructive** |
| `tiktools cloud restore` | restore the cloud session |
| `tiktools cloud backup create [--out FILE]` | write a portable backup archive |
| `tiktools cloud backup preview <file>` | inspect a backup archive |
| `tiktools cloud backup restore <file> [--yes]` | restore a backup archive **destructive** |

### `tiktools publisher` — cloud publisher flow (requires sign-in)

| Command | Summary |
| --- | --- |
| `tiktools publisher list` | list publisher namespaces |
| `tiktools publisher create <namespace>` | create a publisher namespace |
| `tiktools publisher delete <namespace-or-publisher-id>` | delete a namespace or publisher **destructive** |
| `tiktools publisher keys <publisher-id>` | list signing keys of a publisher |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | enroll a publisher key; reads the backend password from stdin |
| `tiktools publisher upload <publisher-id> <path>` | upload a plugin package |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | submit an upload for review |
| `tiktools publisher release <release-id>` | create a release |
| `tiktools publisher publish <release-id>` | publish a release |

### `tiktools registry` — package registries and the cloud catalog

| Command | Summary |
| --- | --- |
| `tiktools registry list` | list configured registries |
| `tiktools registry refresh` | refresh registry metadata |
| `tiktools registry package <id>` | show one registry package |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | search the cloud catalog |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | cloud catalog summaries page |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | versions of one cloud catalog package |
| `tiktools registry cloud-pagination` | pagination capabilities of the cloud catalog |
| `tiktools registry defaults` | default registry configuration |

### `tiktools processor` — event processors

| Command | Summary |
| --- | --- |
| `tiktools processor list` | list processors |
| `tiktools processor status` | processor pipeline status |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | run one processor against a sample event |

### `tiktools live` — TikTok LIVE connection

| Command | Summary |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | connect to a live room |
| `tiktools live pick --session-cookie <cookie>` | pick a live room interactively |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | discover live channels; works without a session |
| `tiktools live disconnect` | disconnect from the current room **destructive** |
| `tiktools live status` | current connection status |

### `tiktools points` — viewer loyalty points

| Command | Summary |
| --- | --- |
| `tiktools points config get` | read the points configuration |
| `tiktools points config set k=v...` | update the points configuration |
| `tiktools points viewer get <id>` | read one viewer's balance |
| `tiktools points adjust <id> <delta>` | add or subtract points for a viewer |
| `tiktools points leaderboard [--limit N]` | top viewers by balance |
| `tiktools points reset [id]` | reset one viewer or every balance **destructive** |

### `tiktools ranking` — viewer ranking definitions and state

| Command | Summary |
| --- | --- |
| `tiktools ranking list` | list ranking definitions |
| `tiktools ranking metrics` | metrics available to ranking definitions |
| `tiktools ranking create --record <json&#124;@file>` | create a ranking definition |
| `tiktools ranking update --record <json&#124;@file>` | update a ranking definition |
| `tiktools ranking get <id>` | read one ranking |
| `tiktools ranking snapshot <id>` | current standings of one ranking |
| `tiktools ranking delete <id>` | delete a ranking **destructive** |
| `tiktools ranking reset <id>` | reset accumulated state **destructive** |
| `tiktools ranking pause <id>` | pause accumulation |
| `tiktools ranking resume <id>` | resume accumulation |
| `tiktools ranking contribute --record <json&#124;@file>` | add an external contribution |

### `tiktools competition` — timed viewer competitions

| Command | Summary |
| --- | --- |
| `tiktools competition list` | list competitions |
| `tiktools competition get <id>` | read one competition |
| `tiktools competition create --record <json&#124;@file>` | create a competition |
| `tiktools competition update --record <json&#124;@file>` | update a competition |
| `tiktools competition delete <id>` | delete a competition **destructive** |
| `tiktools competition state <id>` | current competition state |
| `tiktools competition results <id>` | final or intermediate results |
| `tiktools competition action --record <json&#124;@file>` | record a competition action |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | flag an action as uncertain |
| `tiktools competition resolve-action --record <json&#124;@file>` | resolve an uncertain action |
| `tiktools competition start <id>` | start a competition |
| `tiktools competition finish <id>` | finish a competition **destructive** |
| `tiktools competition candidates <id>` | winner candidates |
| `tiktools competition select-winner --record <json&#124;@file>` | select the winner |
| `tiktools competition pause <id>` | pause a competition |
| `tiktools competition resume <id>` | resume a competition |
| `tiktools competition cancel <id>` | cancel a competition **destructive** |

### `tiktools automation` — event and action automation rules

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

### `tiktools globals` — shared key/value globals across plugins

| Command | Summary |
| --- | --- |
| `tiktools globals list` | list every global key |
| `tiktools globals get <key>` | read one global |
| `tiktools globals set <key> <value>` | write one global |
| `tiktools globals delete <key>` | delete one global **destructive** |

### `tiktools workflow` — visual workflow graphs

| Command | Summary |
| --- | --- |
| `tiktools workflow list` | list saved workflow graphs |
| `tiktools workflow get <id>` | read one graph |
| `tiktools workflow save --record json` | create or replace a graph |
| `tiktools workflow delete <id>` | delete a graph **destructive** |
| `tiktools workflow enable <id>` | enable a graph |
| `tiktools workflow disable <id>` | disable a graph |

### `tiktools media` — media file validation and playback

| Command | Summary |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | validate a media file before use in actions |

### `tiktools system` — host information and diagnostics

| Command | Summary |
| --- | --- |
| `tiktools system info` | host version and runtime information |
| `tiktools system health` | component health report |
| `tiktools system snapshot` | full state snapshot for debugging |
| `tiktools system doctor` | diagnose common setup problems |
| `tiktools system request-input-access` | ask the desktop user for input access |

### `tiktools template` — reusable automation templates and profiles

| Command | Summary |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | import a template as automation rules |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | export an event rule as a template |
| `tiktools template list` | list templates in the gallery |
| `tiktools template delete <template-id>` | delete a template **destructive** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | import a whole automation profile |
| `tiktools template profile-export --out <file> [--name n]` | export the automation profile |

### `tiktools api` — generic agent-oriented control surface over host methods

| Command | Summary |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | list every method live from the host registry |
| `tiktools api schema <method> [--format json&#124;human]` | one method's description, flags and schemas |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validate and execute one method; destructive methods need --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | stream NDJSON domain events |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | search the bounded feed journal |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verify the control surface against the host |

### `tiktools host` — headless host runtimes and the raw rpc passthrough

| Command | Summary |
| --- | --- |
| `tiktools host --stdio [--events]` | headless host on stdio; streams events with --events |
| `tiktools host --ipc` | headless host on local IPC; refuses when a host is running |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | untyped passthrough to any host method |
| `tiktools rpc --stdio [--events]` | NDJSON request/response loop on stdio |

### `tiktools web` — local browser adapter (requires the local-web feature)

| Command | Summary |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | serve the local browser adapter; host --ipc must be running |
| `tiktools web --print-code [--port N]` | print the active pairing code without restarting |
| `tiktools web --reset-session [--port N]` | revoke every paired browser and print a fresh code **destructive** |

### Conventions

- **host resolution**: commands run against the running host over local IPC; when no host is running they fail with host_unavailable instead of starting a second runtime. Use `tiktools --json system info` as the lightweight probe: exit 0 means a host answered, exit 3 means the IPC host is unavailable. --standalone runs the command against an isolated in-process runtime instead.
- **output**: stdout carries the result; diagnostics go to stderr. --json prints the raw result object; without it, lists print one line per item. json values may use @path to read from a file. Human help follows the display locale: TIKTOOLS_LANG overrides the LC_ALL/LC_MESSAGES/LANG chain, and machine output stays locale-independent.
- **exit codes**: 0 ok, 1 operation error, 2 usage error, 3 transport failure.
- **secrets**: live connect/pick read the session cookie from --session-cookie or the TIKTOOLS_SESSION_COOKIE environment variable (the flag wins); the value never appears in diagnostics. api call redacts secrets from its output unless --secrets-visible is passed, and refuses destructive methods without --confirm.
