| 命令 | 摘要 |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | 列出自动化规则 |
| `tiktools automation context [--event-type t]` | 规则可用的模板上下文 |
| `tiktools automation get <id> [--kind event&#124;action]` | 读取一条规则 |
| `tiktools automation create --record json [--kind event&#124;action]` | 创建规则 |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | 更新规则 |
| `tiktools automation delete <id> [--kind event&#124;action]` | 删除规则 **破坏性** |
| `tiktools automation enable <id> [--kind event&#124;action]` | 启用规则 |
| `tiktools automation disable <id> [--kind event&#124;action]` | 禁用规则 |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | 对示例事件空跑一条规则 |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | 手动触发触发器 |
| `tiktools automation moderation-penalty --points <delta>` | 以积分施加管理处罚 |
