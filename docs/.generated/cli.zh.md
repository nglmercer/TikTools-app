`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — 安装和管理插件

| 命令 | 摘要 |
| --- | --- |
| `tiktools plugin list` | 列出已安装的插件 |
| `tiktools plugin get <id>` | 读取一个插件 |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | 从文件、id@version 或 url 安装 |
| `tiktools plugin uninstall <id> [--purge]` | 卸载插件 **破坏性** |
| `tiktools plugin search <query> [--category c] [--page N]` | 在注册表中搜索软件包 |
| `tiktools plugin inspect <spec>` | 安装前检查软件包规格 |
| `tiktools plugin outdated` | 列出有可用更新的已安装插件 |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | 更新一个插件 |
| `tiktools plugin update --all [--yes] [--no-wait]` | 更新所有过时的插件 |
| `tiktools plugin rollback <id> [version] [--no-wait]` | 回滚到以前的版本 **破坏性** |
| `tiktools plugin versions <id>` | 列出插件的可用版本 |
| `tiktools plugin pin <id>` | 将插件固定到当前版本 |
| `tiktools plugin unpin <id>` | 移除版本固定 |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | 设置通道和自动更新策略 |
| `tiktools plugin jobs` | 列出软件包任务 |
| `tiktools plugin job <job-id>` | 检查一个软件包任务 |
| `tiktools plugin cancel <job-id>` | 取消正在运行的软件包任务 **破坏性** |
| `tiktools plugin enable <id>` | 启用插件 |
| `tiktools plugin disable <id>` | 禁用插件 |
| `tiktools plugin start <id>` | 启动插件 |
| `tiktools plugin stop <id>` | 停止插件 **破坏性** |
| `tiktools plugin settings get <id>` | 读取插件设置 |
| `tiktools plugin settings set <id> k=v...` | 写入插件设置 |
| `tiktools plugin settings reset <id>` | 重置插件设置 **破坏性** |
| `tiktools plugin health <id>` | 插件健康检查 |
| `tiktools plugin options <action-type/field>` | 动作类型或字段的选项 |
| `tiktools plugin action <type> [--config json] [--live]` | 运行插件动作 |
| `tiktools plugin commands` | 列出已安装插件贡献的命令 |
| `tiktools plugin help [id]` | 描述一个贡献的命令 |
| `tiktools plugin <id> <command> [--params json] [--live]` | 调用插件贡献的命令 |

### `tiktools cloud` — 云账户和便携式备份/恢复（用 TIKTOOLS_CLOUD_ORIGIN 覆盖）

| 命令 | 摘要 |
| --- | --- |
| `tiktools cloud status` | 云账户状态 |
| `tiktools cloud login` | 通过系统浏览器登录 |
| `tiktools cloud logout` | 退出登录 **破坏性** |
| `tiktools cloud restore` | 恢复云会话 |
| `tiktools cloud backup create [--out FILE]` | 写入便携式备份存档 |
| `tiktools cloud backup preview <file>` | 检查备份存档 |
| `tiktools cloud backup restore <file> [--yes]` | 恢复备份存档 **破坏性** |

### `tiktools publisher` — 云发布者流程（需要登录）

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

### `tiktools registry` — 软件包注册表和云目录

| 命令 | 摘要 |
| --- | --- |
| `tiktools registry list` | 列出已配置的注册表 |
| `tiktools registry refresh` | 刷新注册表元数据 |
| `tiktools registry package <id>` | 显示一个注册表软件包 |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | 搜索云目录 |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | 云目录摘要页 |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | 云目录软件包的版本 |
| `tiktools registry cloud-pagination` | 云目录的分页能力 |
| `tiktools registry defaults` | 默认注册表配置 |

### `tiktools processor` — 事件处理器

| 命令 | 摘要 |
| --- | --- |
| `tiktools processor list` | 列出处理器 |
| `tiktools processor status` | 处理器管道状态 |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | 对示例事件运行一个处理器 |

### `tiktools live` — TikTok LIVE 连接

| 命令 | 摘要 |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | 连接到直播房间 |
| `tiktools live pick --session-cookie <cookie>` | 交互式选择直播房间 |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | 发现直播频道；无需会话即可工作 |
| `tiktools live disconnect` | 断开当前房间的连接 **破坏性** |
| `tiktools live status` | 当前连接状态 |

### `tiktools points` — 观众忠诚度积分

| 命令 | 摘要 |
| --- | --- |
| `tiktools points config get` | 读取积分配置 |
| `tiktools points config set k=v...` | 更新积分配置 |
| `tiktools points viewer get <id>` | 读取一位观众的余额 |
| `tiktools points adjust <id> <delta>` | 为观众增加或减少积分 |
| `tiktools points leaderboard [--limit N]` | 按余额排名的观众 |
| `tiktools points reset [id]` | 重置一位观众或所有余额 **破坏性** |

### `tiktools ranking` — 观众排名定义和状态

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

### `tiktools competition` — 定时观众竞赛

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

### `tiktools automation` — 事件和动作自动化规则

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

### `tiktools globals` — 插件间共享的键/值全局变量

| 命令 | 摘要 |
| --- | --- |
| `tiktools globals list` | 列出所有全局键 |
| `tiktools globals get <key>` | 读取一个全局变量 |
| `tiktools globals set <key> <value>` | 写入一个全局变量 |
| `tiktools globals delete <key>` | 删除一个全局变量 **破坏性** |

### `tiktools workflow` — 可视化工作流图

| 命令 | 摘要 |
| --- | --- |
| `tiktools workflow list` | 列出已保存的工作流图 |
| `tiktools workflow get <id>` | 读取一个图 |
| `tiktools workflow save --record json` | 创建或替换图 |
| `tiktools workflow delete <id>` | 删除图 **破坏性** |
| `tiktools workflow enable <id>` | 启用图 |
| `tiktools workflow disable <id>` | 禁用图 |

### `tiktools media` — 媒体文件验证和播放

| 命令 | 摘要 |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | 在动作中使用前验证媒体文件 |

### `tiktools system` — 主机信息和诊断

| 命令 | 摘要 |
| --- | --- |
| `tiktools system info` | 主机版本和运行时信息 |
| `tiktools system health` | 组件健康报告 |
| `tiktools system snapshot` | 用于调试的完整状态快照 |
| `tiktools system doctor` | 诊断常见的设置问题 |
| `tiktools system request-input-access` | 向桌面用户请求输入访问权限 |
| `tiktools system shutdown` | 关闭运行中的主机 **破坏性** |

### `tiktools template` — 可重用的自动化模板和配置文件

| 命令 | 摘要 |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | 将模板导入为自动化规则 |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | 将事件规则导出为模板 |
| `tiktools template list` | 列出模板库中的模板 |
| `tiktools template delete <template-id>` | 删除模板 **破坏性** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | 导入完整的自动化配置文件 |
| `tiktools template profile-export --out <file> [--name n]` | 导出自动化配置文件 |

### `tiktools api` — 面向代理的通用控制面，覆盖主机方法

| 命令 | 摘要 |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | 从主机注册表实时列出所有方法 |
| `tiktools api schema <method> [--format json&#124;human]` | 一个方法的描述、标志和架构 |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | 验证并执行一个方法；破坏性方法需要 --confirm |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | 流式传输 NDJSON 域事件 |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | 搜索有界动态日志 |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | 针对主机验证控制面 |

### `tiktools host` — 无界面主机运行时和原始 rpc 直通

| 命令 | 摘要 |
| --- | --- |
| `tiktools host --stdio [--events]` | stdio 上的无界面主机；用 --events 流式传输事件 |
| `tiktools host --ipc` | 本地 IPC 上的无界面主机；有主机运行时拒绝启动 |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | 到任何主机方法的无类型直通 |
| `tiktools rpc --stdio [--events]` | stdio 上的 NDJSON 请求/响应循环 |

### `tiktools web` — 本地浏览器适配器（需要 local-web 特性）

| 命令 | 摘要 |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | 服务本地浏览器适配器；host --ipc 必须在运行 |
| `tiktools web --print-code [--port N]` | 不重启打印当前配对码 |
| `tiktools web --reset-session [--port N]` | 撤销所有已配对浏览器并打印新配对码 **破坏性** |

### 约定

- **主机解析**: 命令通过本地 IPC 对运行中的主机执行；没有主机运行时，它们以 host_unavailable 失败，而不是启动第二个运行时。使用 `tiktools --json system info` 作为轻量探针：退出码 0 表示主机已响应，退出码 3 表示 IPC 主机不可用。--standalone 改为对隔离的进程内运行时执行命令。
- **输出**: stdout 承载结果；诊断输出到 stderr。--json 打印原始结果对象；不带它时，列表每项打印一行。json 值可以使用 @path 从文件读取。人类可读的帮助跟随显示语言环境：TIKTOOLS_LANG 覆盖 LC_ALL/LC_MESSAGES/LANG 链，机器输出保持与语言环境无关。
- **退出码**: 0 成功，1 操作错误，2 用法错误，3 传输失败。
- **密钥**: live connect/pick 从 --session-cookie 或 TIKTOOLS_SESSION_COOKIE 环境变量读取会话 cookie（标志优先）；该值从不出现在诊断中。api call 除非传入 --secrets-visible，否则会在输出中隐去秘密，并且没有 --confirm 时拒绝破坏性方法。
