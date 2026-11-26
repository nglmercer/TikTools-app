| Comando | Resumo |
| --- | --- |
| `tiktools host --stdio [--events]` | host sem interface em stdio; transmite eventos com --events |
| `tiktools host --ipc` | host sem interface em IPC local; recusa quando já há um host em execução |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | passagem direta sem tipos para qualquer método do host |
| `tiktools rpc --stdio [--events]` | laço de requisição/resposta NDJSON em stdio |
