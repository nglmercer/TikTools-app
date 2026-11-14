---
title: "按功能区域审查变更"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f875a22e66dd822d7ea1344188d8b7b087a23a777e072240a4a3c7c57969f995"
translationRevision: "353195b2b412e47763fc0520f40936727eb19f0c56141c8e581b0c811c0e31c7"
order: 20
---

# 按功能区域审查变更 {#review-changes-by-area}

## 目标与前置条件 {#goal-and-prerequisites}

重置、发送或采纳数据前，了解 **更改** 面板究竟比较什么。你需要已保存的本地内容。Cloud 操作还需要受支持的宿主、对应的同步模式及[已登录的账户](account.md)。名称相同的面板并不都采用相同的传输方式。

## 在本地审查事件与操作 {#review-events-and-actions-locally}

1. 打开 **自动化**，选择事件或操作，再打开 **更改**。
2. 阅读提示：此区域将当前定义与本设备保留的参考副本进行比较。首次加载访问会建立该参考，因此没有变更也属正常。
3. 选择新增、修改或删除的记录，检查前后值。
4. 仅在希望将当前分区作为新本地参考时选择 **标记为已审查**。这不会上传定义，也不会批准受委托的执行。
5. 仅在确实要恢复参考状态时使用 **重置为已同步版本**。对于新增记录，重置会移除它；对于修改或删除的记录，会恢复保留的定义。如有需要，请先导出。

此版本的事件/操作适配器没有用于单条定义的 Cloud 集合。比较结果干净既不代表完成 Cloud 备份，也不能证明另一台电脑收到了编辑。

## 明确审查变量与插件 {#review-variables-and-plugins-explicitly}

变量有独立的原生审查式共享流程：先本地保存，再获取候选项、检查比较，最后明确发送、应用/采纳或撤回。秘密值和被阻止的值保持遮蔽，不能通过该路径发送。本地保存从不上传。请参阅[变量](../automations/variables.md)。

插件更改使用其支持的同步工作区管理插件选择及暂存设置。发布按分区进行，而非逐行单独发送。设置只显示已更改的键名，不暴露值；冲突时可选择本地版或 Cloud 版，不支持任意字段合并。发布前请阅读分区操作及选中的同步模式。

Cloud 叠加层采用[库/编辑器](../overlays/index.md)的创建与修订流程。此基础版本不提供共享叠加层更改适配器。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

本地自动化比较和按能力开放的原生共享，在各自区域中可用。产生远程副作用的操作依赖服务，并不表示已通过生产验收。审查过期时，刷新并重新比较。结果未确认时，先检查状态或日志，再创建另一个操作。不要将超时当作没有发生变更的证据。

<DemoFrame scenario="cloud" locale="zh" tour="changes" title="演示：Cloud 变更" />

## 运行细节 {#operational-details}

准备或执行 v1 → v2 迁移前，先备份本地数据，并协调所有设备使用兼容版本。在 setup 依次准备、检查兼容性和预览；分别查看选择、待处理、冲突及仅 v1 项目数量，不要假定全部已同步。要求写入 epoch 头对账户全部 v2 写入不可逆，可能拒绝旧设备。屏障超时代表结果不明，重复调用会再次推进 epoch。后续操作前刷新迁移/epoch 状态和预览；读取 epoch 不证明旧写入已经完成。

恢复依模式而定：Events/Actions Changes 只比较本地参考，不写 Cloud。已审核 Variables/Plugins 比较过期时重新加载，核对当前修订、账户和清单。缺少插件时使用 setup 清单审核/修复，再生成预览。保留的不明意图应保存 ID/证据并查看历史/状态；技术恢复需要重新审核冻结条目及 Cloud 修订。恢复该意图本身不发送远程写入。无法核对状态时停止，不要创建替代变更绕过不确定性。
