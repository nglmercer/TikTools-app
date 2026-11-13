---
title: "创建第一条日志规则"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# 创建第一条日志规则 {#create-your-first-logging-rule}

## 目标与前置条件 {#goal-and-prerequisites}

通过一行本地日志验证从事件到操作的完整路径。你需要运行中的桌面宿主，并能访问 **自动化**。无需 Cloud 账户、音频服务或外部端点。请使用虚构或不敏感的聊天内容，因为日志会记录渲染后的消息。

## 创建操作 {#create-the-action}

1. 在要使用的配置中打开 **自动化 → 操作**。
2. 创建一个写入日志的操作（`core.log`），命名为 `聊天检查`。
3. 将消息字段设为以下模板，然后保存并启用该操作：

```text
聊天检查：{{ event.user.uniqueId }} 说 {{ event.data.comment }}
```

这些是当前标准化聊天事件的字段路径。`event.data.comment` 是消息，`event.user.uniqueId` 是观众账号名。显示名称使用另一个字段。

## 创建事件 {#create-the-event}

1. 打开 **事件**，创建 `聊天检查事件`，并选择 `tiktok.chat` 作为触发器。
2. 仅关联 `聊天检查`，保持常规的全部操作模式。
3. 首次样例测试时不设置筛选条件。将全局冷却时间设为 `10` 秒，以便之后限制真实流量。
4. 保存并查看显示的变更预览。确认仅关联日志操作后，使用 **运行测试**，检查返回的运行记录及日志文本。宿主可提供样例；样例结果不能证明真实 TikTok 连接正常。
5. 添加条件：字段 `event.data.comment`，运算符等于（`eq`），值为 `!hello`。再次保存并明确启用该事件。
6. 连接到你有权测试的[直播](../live/connections.md)。在[事件流](../live/feed.md)中观察一条新的 `!hello` 聊天消息，再于冷却条件允许后检查自动化历史。

## 预期结果与状态 {#expected-result-and-status}

本地日志操作及聊天触发器均可使用。样例应产生可读的日志结果；匹配的真实聊天应产生包含实际账号名和消息的运行记录。其他聊天文本应无法通过精确匹配条件。

## 恢复方法 {#recovery}

添加筛选后，如果样例不再匹配，可能是它提供的评论并非 `!hello`。请检查真实事件字段、当前配置及启用状态，不要反复触发。测试会执行实际操作；此规则仅写日志，便于预测效果。完成后禁用事件，或继续阅读[实用配方](recipes.md)。

<DemoFrame scenario="automation" locale="zh" tour="automation" title="演示：自动化" />
