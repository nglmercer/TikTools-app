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
