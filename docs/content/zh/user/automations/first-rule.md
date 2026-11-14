---
title: "创建第一条日志规则"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "13b4b7b3cb8eba95257ec9738934813266414f504eec8b43090b73e74ad50c10"
translationRevision: "ffb3980c52e6ce4cae9eb068948c03410c83d7489a501213fdc511ee258035dd"
order: 20
---

# 创建第一条日志规则 {#create-your-first-logging-rule}

## 目标与前置条件 {#goal-and-prerequisites}

将聊天事件转为一行易读的本地日志。你需要运行中的桌面宿主，并能访问 **自动化**。无需 Cloud 账户、音频服务或外部端点。

<DocCallout title="先得到可检查的本地结果" tone="tip">

让第一条规则只写日志，便于查看结果。请使用虚构或不敏感的聊天内容，因为日志会记录渲染后的消息。交互式示例使用预先准备好的工作区，可随时重置。

</DocCallout>

## 创建操作 {#create-the-action}

<DocSteps>

1. **打开操作编辑器。** 在要使用的配置方案中打开 **自动化 → 操作**。
2. **为操作命名。** 创建一个 **写入日志** 操作（`core.log`），命名为 `聊天检查`。
3. **设置消息。** 将消息字段设为以下模板，然后保存并启用该操作。

</DocSteps>

```text
聊天检查：{{ event.user.uniqueId }} 说 {{ event.data.comment }}
```

`event.data.comment` 是标准化的聊天消息，`event.user.uniqueId` 是观众账号名。显示名称使用另一个字段。

<DemoFrame scenario="automation" locale="zh" panel="automation" title="探索自动化编辑器" />

## 创建事件 {#create-the-event}

<DocSteps>

1. **选择触发器。** 打开 **事件**，创建 `聊天检查事件`，并选择 `tiktok.chat`。
2. **关联操作。** 仅关联 `聊天检查`，保持常规的全部操作模式。
3. **建立测试基准。** 首次样例测试时不设置筛选条件。将全局冷却时间设为 `10` 秒，以便之后限制真实流量。
4. **测试结果。** 保存并查看显示的变更预览。确认仅关联日志操作后，使用 **运行测试**，检查返回的运行记录及日志文本。宿主提供的样例不能证明真实 TikTok 连接正常。
5. **收窄规则。** 添加条件：字段 `event.data.comment`，运算符等于（`eq`），值为 `!hello`。再次保存并明确启用该事件。
6. **检查真实聊天。** 连接到你有权测试的[直播](../live/connections.md)。在[事件流](../live/feed.md)中观察一条新的 `!hello` 聊天消息，再于冷却条件允许后检查自动化历史。

</DocSteps>

## 预期结果与状态 {#expected-result-and-status}

本地日志操作及聊天触发器均可使用。样例应产生可读的日志结果。匹配的真实聊天应产生包含实际观众账号名和消息的运行记录；其他聊天文本应无法通过精确匹配条件。

<DocCallout title="检查完整执行路径" tone="info">

确认当前配置方案、操作和事件的启用状态、消息是否匹配，以及冷却时间是否已过。仅在事件流中看到聊天，不能证明自动化已执行。

</DocCallout>

## 恢复方法 {#recovery}

添加筛选后，如果样例不再匹配，可能是它提供的评论并非 `!hello`。请检查真实事件字段、当前配置方案及启用状态，不要反复触发。测试会执行实际操作；此规则仅写日志，便于预测效果。完成后禁用事件，或继续阅读[实用配方](recipes.md)。
