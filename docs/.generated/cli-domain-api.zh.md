| 命令 | 摘要 |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | 从主机注册表实时列出所有方法 |
| `tiktools api schema <method> [--format json&#124;human]` | 一个方法的描述、标志和架构 |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | 验证并执行一个方法；破坏性方法需要 --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | 流式传输 NDJSON 域事件 |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | 搜索有界动态日志 |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | 针对主机验证控制面 |
