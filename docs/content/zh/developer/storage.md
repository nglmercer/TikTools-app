---
title: "持久化与持久恢复"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7ec9d94e484af17e8e82aaaa2122cc47ecd206cf9306047795d6aaa72eea2edb"
translationRevision: "1eebde56f068e9c54db4e6201a19caec1b093f1d229bd33d4953e7a027266391"
order: 60
---

# 持久化与持久恢复 {#persistence-and-durable-recovery}

## 目标与前置条件 {#goal-and-prerequisites}

通过所属服务修改持久状态，并保留可恢复结果。使用可丢弃配置，复制数据库前关闭其宿主，编辑版本化资源前阅读[控制元数据](control-api.md)。`TIKTOOLS_HOME` 选择应用根目录；`AppPaths` 解析数据及私有存储位置。

## 识别存储 {#identify-the-store}

`crates/tiktools-core/src/paths.rs` 定义 `data/tiktok-points.db`、`data/tiktok-automation.db` 和 `data/tiktok-analytics.db`。SQLite 服务模块管理各自 schema。积分/观众和自动化定义拥有独立业务账本；统计是单独数据库，旧的双数据库图遗漏了它。

启动时，仅在目标不存在的情况下，才可将检出目录中的旧数据库复制到平台应用数据目录，绝不覆盖现有目标。原生执行变化期间，JSON 定义仍保持可读。不要通过删除用户配置或从 Vue 重写数据表来实现迁移。

## 进行可恢复的修改 {#make-a-recoverable-change}

1. 找到服务事务及其 schema/迁移测试。
2. 在领域支持的情况下，将业务变更、修订和必需日志/发件箱预留写入同一事务。
3. 删除时保留修订墓碑。复用的记录 ID 不得接受删除前的编辑修订。
4. 通过现有发件箱发布已提交的失效元数据；投递重试不得重复底层操作。
5. 中断后重新打开同一私有测试主目录，并在尝试其他修改前解决原始命令。

对已运行测试宿主的有用只读检查：

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## 预期结果与状态 {#expected-result-and-status}

受支持的持久化工作流程、积分和所有者日志会跨重启保留原始结果。回执描述历史命令，当前状态应单独读取。Cloud 工作区存储添加来源/账户分区及代次检查，不会替代本地 SQLite 的权威性。并非所有外部效果或领域都具备同样的持久恢复覆盖。

## 恢复方法 {#recovery}

离线备份时，将 SQLite 文件与所有活动 WAL/SHM 文件一同保留。UI 超时不能证明已回滚。遇到 `revision_conflict`，保留草稿并比较新的读取结果。外部效果不确定时，检查保留的效果记录，不要重放事件流。各自边界见[路径权威实现](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs)及 [Cloud 恢复](cloud.md)。

## 运行细节 {#operational-details}

试验或备份前，使用 `tiktools api call system.info --json` 查看实际路径。`TIKTOOLS_HOME` 选择根目录；`TIKTOOLS_DATA_DIR`、`TIKTOOLS_PLUGINS_DIR`、`TIKTOOLS_PLUGIN_DATA_DIR`、`TIKTOOLS_PLUGIN_STORE_DIR`、`TIKTOOLS_LOG_DIR`、`TIKTOOLS_TEMP_DIR` 分别覆盖派生目录。相对路径基于启动工作目录。单独隔离 home 不会消除继承的覆盖变量，应先检查或取消它们。

Analytics 保存聚合数据并使用独立写入器：当前队列为 1,024，批次为 128，刷新期限 10 秒。数据库旁的 `*.pending.json` 路径由该数据库路径派生。启动时按批次 ID 重放保留内容并移除恢复文件，之后 `ready` 才允许新工作。恢复文件不可读或超过 16 MiB、数据库不可用或无法移除恢复文件时，保持禁止接收。请保留数据库及恢复文件，检查 `ready`、待处理计数和错误，修复前备份。不要删除待处理证据来掩盖故障；积分日志具有独立持久性契约。

`system.info` 报告实际 home/dataDir；其他目录需按宿主真实启动环境解析，不能只查看另一终端。`system.health` 提供 Analytics ready/待处理及 LIVE 入口计数。
