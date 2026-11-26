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
