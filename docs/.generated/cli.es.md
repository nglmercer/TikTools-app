`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — instalar y gestionar plugins

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

### `tiktools cloud` — cuenta cloud y copia/restauración portátil (anula con TIKTOOLS_CLOUD_ORIGIN)

| Comando | Resumen |
| --- | --- |
| `tiktools cloud status` | estado de la cuenta cloud |
| `tiktools cloud login` | iniciar sesión mediante el navegador del sistema |
| `tiktools cloud logout` | cerrar sesión **destructivo** |
| `tiktools cloud restore` | restaurar la sesión cloud |
| `tiktools cloud backup create [--out FILE]` | escribir un archivo de copia portátil |
| `tiktools cloud backup preview <file>` | inspeccionar un archivo de copia |
| `tiktools cloud backup restore <file> [--yes]` | restaurar un archivo de copia **destructivo** |

### `tiktools publisher` — flujo de publicador cloud (requiere iniciar sesión)

| Comando | Resumen |
| --- | --- |
| `tiktools publisher list` | listar los espacios de publicador |
| `tiktools publisher create <namespace>` | crear un espacio de publicador |
| `tiktools publisher delete <namespace-or-publisher-id>` | eliminar un espacio o publicador **destructivo** |
| `tiktools publisher keys <publisher-id>` | listar las claves de firma de un publicador |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | inscribir una clave de publicador; lee la contraseña del backend desde stdin |
| `tiktools publisher upload <publisher-id> <path>` | subir un paquete de plugin |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | enviar una subida a revisión |
| `tiktools publisher release <release-id>` | crear una release |
| `tiktools publisher publish <release-id>` | publicar una release |

### `tiktools registry` — registros de paquetes y el catálogo cloud

| Comando | Resumen |
| --- | --- |
| `tiktools registry list` | listar los registros configurados |
| `tiktools registry refresh` | actualizar los metadatos del registro |
| `tiktools registry package <id>` | mostrar un paquete del registro |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | buscar en el catálogo cloud |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | página de resúmenes del catálogo cloud |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | versiones de un paquete del catálogo cloud |
| `tiktools registry cloud-pagination` | capacidades de paginación del catálogo cloud |
| `tiktools registry defaults` | configuración por defecto del registro |

### `tiktools processor` — procesadores de eventos

| Comando | Resumen |
| --- | --- |
| `tiktools processor list` | listar los procesadores |
| `tiktools processor status` | estado de la canalización de procesadores |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | ejecutar un procesador contra un evento de muestra |

### `tiktools live` — conexión con TikTok LIVE

| Comando | Resumen |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | conectar a una sala en vivo |
| `tiktools live pick --session-cookie <cookie>` | elegir una sala en vivo interactivamente |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | descubrir canales en vivo; funciona sin sesión |
| `tiktools live disconnect` | desconectar de la sala actual **destructivo** |
| `tiktools live status` | estado actual de la conexión |

### `tiktools points` — puntos de fidelidad de espectadores

| Comando | Resumen |
| --- | --- |
| `tiktools points config get` | leer la configuración de puntos |
| `tiktools points config set k=v...` | actualizar la configuración de puntos |
| `tiktools points viewer get <id>` | leer el saldo de un espectador |
| `tiktools points adjust <id> <delta>` | sumar o restar puntos a un espectador |
| `tiktools points leaderboard [--limit N]` | espectadores con mayor saldo |
| `tiktools points reset [id]` | restablecer el saldo de un espectador o de todos **destructivo** |

### `tiktools ranking` — definiciones y estado de rankings de espectadores

| Comando | Resumen |
| --- | --- |
| `tiktools ranking list` | listar las definiciones de ranking |
| `tiktools ranking metrics` | métricas disponibles para las definiciones de ranking |
| `tiktools ranking create --record <json&#124;@file>` | crear una definición de ranking |
| `tiktools ranking update --record <json&#124;@file>` | actualizar una definición de ranking |
| `tiktools ranking get <id>` | leer un ranking |
| `tiktools ranking snapshot <id>` | clasificación actual de un ranking |
| `tiktools ranking delete <id>` | eliminar un ranking **destructivo** |
| `tiktools ranking reset <id>` | restablecer el estado acumulado **destructivo** |
| `tiktools ranking pause <id>` | pausar la acumulación |
| `tiktools ranking resume <id>` | reanudar la acumulación |
| `tiktools ranking contribute --record <json&#124;@file>` | añadir una contribución externa |

### `tiktools competition` — competiciones cronometradas de espectadores

| Comando | Resumen |
| --- | --- |
| `tiktools competition list` | listar las competiciones |
| `tiktools competition get <id>` | leer una competición |
| `tiktools competition create --record <json&#124;@file>` | crear una competición |
| `tiktools competition update --record <json&#124;@file>` | actualizar una competición |
| `tiktools competition delete <id>` | eliminar una competición **destructivo** |
| `tiktools competition state <id>` | estado actual de la competición |
| `tiktools competition results <id>` | resultados finales o intermedios |
| `tiktools competition action --record <json&#124;@file>` | registrar una acción de competición |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | marcar una acción como incierta |
| `tiktools competition resolve-action --record <json&#124;@file>` | resolver una acción incierta |
| `tiktools competition start <id>` | iniciar una competición |
| `tiktools competition finish <id>` | finalizar una competición **destructivo** |
| `tiktools competition candidates <id>` | candidatos a ganador |
| `tiktools competition select-winner --record <json&#124;@file>` | seleccionar el ganador |
| `tiktools competition pause <id>` | pausar una competición |
| `tiktools competition resume <id>` | reanudar una competición |
| `tiktools competition cancel <id>` | cancelar una competición **destructivo** |

### `tiktools automation` — reglas de automatización de eventos y acciones

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

### `tiktools globals` — globales clave/valor compartidos entre plugins

| Comando | Resumen |
| --- | --- |
| `tiktools globals list` | listar todas las claves globales |
| `tiktools globals get <key>` | leer un global |
| `tiktools globals set <key> <value>` | escribir un global |
| `tiktools globals delete <key>` | eliminar un global **destructivo** |

### `tiktools workflow` — grafos de workflows visuales

| Comando | Resumen |
| --- | --- |
| `tiktools workflow list` | listar los grafos de workflow guardados |
| `tiktools workflow get <id>` | leer un grafo |
| `tiktools workflow save --record json` | crear o reemplazar un grafo |
| `tiktools workflow delete <id>` | eliminar un grafo **destructivo** |
| `tiktools workflow enable <id>` | activar un grafo |
| `tiktools workflow disable <id>` | desactivar un grafo |

### `tiktools media` — validación y reproducción de archivos multimedia

| Comando | Resumen |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | validar un archivo multimedia antes de usarlo en acciones |

### `tiktools system` — información y diagnóstico del host

| Comando | Resumen |
| --- | --- |
| `tiktools system info` | versión del host e información del runtime |
| `tiktools system health` | informe de salud de los componentes |
| `tiktools system snapshot` | instantánea completa del estado para depuración |
| `tiktools system doctor` | diagnosticar problemas comunes de configuración |
| `tiktools system request-input-access` | pedir acceso de entrada al usuario del escritorio |
| `tiktools system shutdown` | apagar el host en ejecución **destructivo** |

### `tiktools template` — plantillas de automatización y perfiles reutilizables

| Comando | Resumen |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | importar una plantilla como reglas de automatización |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | exportar una regla de eventos como plantilla |
| `tiktools template list` | listar las plantillas de la galería |
| `tiktools template delete <template-id>` | eliminar una plantilla **destructivo** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | importar un perfil de automatización completo |
| `tiktools template profile-export --out <file> [--name n]` | exportar el perfil de automatización |

### `tiktools api` — superficie de control genérica orientada a agentes sobre los métodos del host

| Comando | Resumen |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | listar todos los métodos en vivo del registro del host |
| `tiktools api schema <method> [--format json&#124;human]` | descripción, flags y esquemas de un método |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validar y ejecutar un método; los destructivos necesitan --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | transmitir eventos de dominio NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | buscar en el diario acotado del feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verificar la superficie de control contra el host |

### `tiktools host` — runtimes de host sin interfaz y el paso directo de rpc

| Comando | Resumen |
| --- | --- |
| `tiktools host --stdio [--events]` | host sin interfaz en stdio; transmite eventos con --events |
| `tiktools host --ipc` | host sin interfaz en IPC local; rechaza si ya hay un host en ejecución |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | paso directo sin tipos a cualquier método del host |
| `tiktools rpc --stdio [--events]` | bucle de petición/respuesta NDJSON en stdio |

### `tiktools web` — adaptador de navegador local (requiere la feature local-web)

| Comando | Resumen |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | servir el adaptador de navegador local; host --ipc debe estar en ejecución |
| `tiktools web --print-code [--port N]` | imprimir el código de emparejamiento activo sin reiniciar |
| `tiktools web --reset-session [--port N]` | revocar todos los navegadores emparejados e imprimir un código nuevo **destructivo** |

### Convenciones

- **resolución del host**: los comandos se ejecutan contra el host en ejecución por IPC local; cuando no hay un host activo fallan con host_unavailable en lugar de iniciar un segundo runtime. Usa `tiktools --json system info` como sonda ligera: salida 0 significa que un host respondió, salida 3 que el host IPC no está disponible. --standalone ejecuta el comando contra un runtime aislado en proceso.
- **salida**: stdout lleva el resultado; los diagnósticos van a stderr. --json imprime el objeto de resultado crudo; sin él, las listas imprimen una línea por elemento. Los valores json pueden usar @path para leer desde un archivo. La ayuda humana sigue el locale de visualización: TIKTOOLS_LANG anula la cadena LC_ALL/LC_MESSAGES/LANG, y la salida de máquina se mantiene independiente del locale.
- **códigos de salida**: 0 correcto, 1 error de operación, 2 error de uso, 3 fallo de transporte.
- **secretos**: live connect/pick leen la cookie de sesión de --session-cookie o de la variable de entorno TIKTOOLS_SESSION_COOKIE (el flag gana); el valor nunca aparece en los diagnósticos. api call oculta los secretos de su salida salvo que se pase --secrets-visible, y rechaza los métodos destructivos sin --confirm.
