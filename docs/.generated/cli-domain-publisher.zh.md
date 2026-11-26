| 命令 | 摘要 |
| --- | --- |
| `tiktools publisher list` | 列出发布者命名空间 |
| `tiktools publisher create <namespace>` | 创建发布者命名空间 |
| `tiktools publisher delete <namespace-or-publisher-id>` | 删除命名空间或发布者 **破坏性** |
| `tiktools publisher keys <publisher-id>` | 列出发布者的签名密钥 |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | 登记发布者密钥；从 stdin 读取后端密码 |
| `tiktools publisher upload <publisher-id> <path>` | 上传插件包 |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | 提交上传以供审核 |
| `tiktools publisher release <release-id>` | 创建发布 |
| `tiktools publisher publish <release-id>` | 发布一个 release |
