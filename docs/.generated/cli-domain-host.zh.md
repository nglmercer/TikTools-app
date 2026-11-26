| 命令 | 摘要 |
| --- | --- |
| `tiktools host --stdio [--events]` | stdio 上的无界面主机；用 --events 流式传输事件 |
| `tiktools host --ipc` | 本地 IPC 上的无界面主机；有主机运行时拒绝启动 |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | 到任何主机方法的无类型直通 |
| `tiktools rpc --stdio [--events]` | stdio 上的 NDJSON 请求/响应循环 |
