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
