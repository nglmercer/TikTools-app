---
title: "组织自动化规则"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
translationRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
order: 10
---

# 组织自动化规则 {#organize-automations}

## 目标与前置条件 {#goal-and-prerequisites}

构建小而清晰的规则库，用于响应直播事件。你需要正常运行的桌面宿主。验证上游事件需要真实的[直播连接](../live/connections.md)；插件操作还需要对应插件及服务就绪。

## 理解规则库 {#understand-the-library}

**操作** 是可复用的已配置操作，例如写入日志或播放声音。**事件** 选择触发器、筛选条件、冷却时间及关联操作。配置将同一工作流程的定义归组；一次只能激活一个配置包。运行时[变量](variables.md)提供可供多个操作复用的值。

1. 打开 **自动化**，编辑前先确认当前配置。分别搜索事件和操作，明确自己正在选择哪类记录。
2. 从[第一条日志规则](first-rule.md)开始。先创建操作，再创建引用该操作的事件。
3. 选择当前宿主支持的触发器。使用字段选择器为该触发器添加筛选。多个条件会缩小匹配范围，不会改变收到的载荷。
4. 设置冷却时间及其范围。全局冷却限制所有观众对该规则的触发；按用户冷却则分别跟踪每位观众。
5. 按顺序关联所需操作。常规模式执行选定列表，随机模式从中选择一个操作。延迟操作会推迟后续操作，不会降低直播接收速度。
6. 保存，检查显示的定义变更，然后明确启用准备运行的记录。在运行历史中检查操作名称、结果及错误。

测试可能执行配置中的实际效果。按 **运行测试** 或触发事件前，请阅读选中的操作列表，尤其注意音频、HTTP、积分和插件操作。学习阶段请使用日志。

## 预期结果与状态 {#expected-result-and-status}

本地编辑、执行、模板、配置及历史均可使用。插件触发器仅在对应插件已安装并启用时出现。导入的定义会保持禁用，直到明确激活。事件/操作的 **更改** 面板是[本地参考比较](../cloud/changes.md)，不会自动上传至 Cloud。

## 恢复方法 {#recovery}

没有运行记录时，依次检查当前配置、启用状态、触发器、筛选条件、冷却时间及缺失操作。发生修订冲突时，请读取当前保存的定义并重新审查，不要盲目覆盖他人的更新。大范围修改前，导出已知正常的[配置](templates.md)。
