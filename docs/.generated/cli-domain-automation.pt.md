| Comando | Resumo |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | listar as regras de automação |
| `tiktools automation context [--event-type t]` | contexto de modelo disponível para as regras |
| `tiktools automation get <id> [--kind event&#124;action]` | ler uma regra |
| `tiktools automation create --record json [--kind event&#124;action]` | criar uma regra |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | atualizar uma regra |
| `tiktools automation delete <id> [--kind event&#124;action]` | excluir uma regra **destrutivo** |
| `tiktools automation enable <id> [--kind event&#124;action]` | ativar uma regra |
| `tiktools automation disable <id> [--kind event&#124;action]` | desativar uma regra |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | testar uma regra em seco contra um evento de amostra |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | disparar um gatilho manualmente |
| `tiktools automation moderation-penalty --points <delta>` | aplicar uma penalidade de moderação em pontos |
