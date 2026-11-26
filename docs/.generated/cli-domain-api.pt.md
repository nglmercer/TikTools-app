| Comando | Resumo |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | listar todos os métodos ao vivo do registro do host |
| `tiktools api schema <method> [--format json&#124;human]` | descrição, flags e esquemas de um método |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | validar e executar um método; os destrutivos precisam de --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | transmitir eventos de domínio NDJSON |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | buscar no diário limitado do feed |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | verificar a superfície de controle contra o host |
