| 命令 | 摘要 |
| --- | --- |
| `tiktools competition list` | 列出竞赛 |
| `tiktools competition get <id>` | 读取一个竞赛 |
| `tiktools competition create --record <json&#124;@file>` | 创建竞赛 |
| `tiktools competition update --record <json&#124;@file>` | 更新竞赛 |
| `tiktools competition delete <id>` | 删除竞赛 **破坏性** |
| `tiktools competition state <id>` | 当前竞赛状态 |
| `tiktools competition results <id>` | 最终或中间结果 |
| `tiktools competition action --record <json&#124;@file>` | 记录竞赛动作 |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | 将动作标记为不确定 |
| `tiktools competition resolve-action --record <json&#124;@file>` | 解决不确定的动作 |
| `tiktools competition start <id>` | 开始竞赛 |
| `tiktools competition finish <id>` | 结束竞赛 **破坏性** |
| `tiktools competition candidates <id>` | 获胜者候选人 |
| `tiktools competition select-winner --record <json&#124;@file>` | 选择获胜者 |
| `tiktools competition pause <id>` | 暂停竞赛 |
| `tiktools competition resume <id>` | 恢复竞赛 |
| `tiktools competition cancel <id>` | 取消竞赛 **破坏性** |
