| Comando | Resumo |
| --- | --- |
| `tiktools publisher list` | listar os namespaces de publicador |
| `tiktools publisher create <namespace>` | criar um namespace de publicador |
| `tiktools publisher delete <namespace-or-publisher-id>` | excluir um namespace ou publicador **destrutivo** |
| `tiktools publisher keys <publisher-id>` | listar as chaves de assinatura de um publicador |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | inscrever uma chave de publicador; lê a senha do backend a partir de stdin |
| `tiktools publisher upload <publisher-id> <path>` | enviar um pacote de plugin |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | submeter um envio para revisão |
| `tiktools publisher release <release-id>` | criar uma release |
| `tiktools publisher publish <release-id>` | publicar uma release |
