| 命令 | 摘要 |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | 连接到直播房间 |
| `tiktools live pick --session-cookie <cookie>` | 交互式选择直播房间 |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | 发现直播频道；无需会话即可工作 |
| `tiktools live disconnect` | 断开当前房间的连接 **破坏性** |
| `tiktools live status` | 当前连接状态 |
