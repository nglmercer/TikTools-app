| 命令 | 摘要 |
| --- | --- |
| `tiktools ranking list` | 列出排名定义 |
| `tiktools ranking metrics` | 排名定义可用的指标 |
| `tiktools ranking create --record <json&#124;@file>` | 创建排名定义 |
| `tiktools ranking update --record <json&#124;@file>` | 更新排名定义 |
| `tiktools ranking get <id>` | 读取一个排名 |
| `tiktools ranking snapshot <id>` | 排名的当前排名情况 |
| `tiktools ranking delete <id>` | 删除排名 **破坏性** |
| `tiktools ranking reset <id>` | 重置累积状态 **破坏性** |
| `tiktools ranking pause <id>` | 暂停累积 |
| `tiktools ranking resume <id>` | 恢复累积 |
| `tiktools ranking contribute --record <json&#124;@file>` | 添加外部贡献 |
