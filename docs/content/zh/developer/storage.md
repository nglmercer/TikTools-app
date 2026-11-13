---
title: "持久化与持久恢复"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
translationRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
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
