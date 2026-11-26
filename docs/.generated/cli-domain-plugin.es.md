| Comando | Resumen |
| --- | --- |
| `tiktools plugin list` | listar los plugins instalados |
| `tiktools plugin get <id>` | leer un plugin |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | instalar desde un archivo, id@version o url |
| `tiktools plugin uninstall <id> [--purge]` | desinstalar un plugin **destructivo** |
| `tiktools plugin search <query> [--category c] [--page N]` | buscar paquetes en el registro |
| `tiktools plugin inspect <spec>` | inspeccionar una especificación de paquete antes de instalar |
| `tiktools plugin outdated` | listar los plugins instalados con actualizaciones disponibles |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | actualizar un plugin |
| `tiktools plugin update --all [--yes] [--no-wait]` | actualizar todos los plugins desactualizados |
| `tiktools plugin rollback <id> [version] [--no-wait]` | volver a una versión anterior **destructivo** |
| `tiktools plugin versions <id>` | listar las versiones disponibles de un plugin |
| `tiktools plugin pin <id>` | fijar un plugin a su versión actual |
| `tiktools plugin unpin <id>` | quitar la fijación de versión |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | establecer el canal y la política de autoactualización |
| `tiktools plugin jobs` | listar los trabajos de paquetes |
| `tiktools plugin job <job-id>` | inspeccionar un trabajo de paquete |
| `tiktools plugin cancel <job-id>` | cancelar un trabajo de paquete en curso **destructivo** |
| `tiktools plugin enable <id>` | activar un plugin |
| `tiktools plugin disable <id>` | desactivar un plugin |
| `tiktools plugin start <id>` | iniciar un plugin |
| `tiktools plugin stop <id>` | detener un plugin **destructivo** |
| `tiktools plugin settings get <id>` | leer la configuración de un plugin |
| `tiktools plugin settings set <id> k=v...` | escribir la configuración de un plugin |
| `tiktools plugin settings reset <id>` | restablecer la configuración de un plugin **destructivo** |
| `tiktools plugin health <id>` | comprobación de salud de un plugin |
| `tiktools plugin options <action-type/field>` | opciones de un tipo de acción o campo |
| `tiktools plugin action <type> [--config json] [--live]` | ejecutar una acción de plugin |
| `tiktools plugin commands` | listar los comandos aportados por los plugins instalados |
| `tiktools plugin help [id]` | describir un comando aportado |
| `tiktools plugin <id> <command> [--params json] [--live]` | invocar un comando aportado por un plugin |
