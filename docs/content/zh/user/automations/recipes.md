---
title: "调整实用自动化配方"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
translationRevision: "449d85fd6184648ff673b6248625fdc5b417a6f9f76266f83181b4ea3552774f"
order: 30
---

# 调整实用自动化配方 {#adapt-practical-automation-recipes}

## 目标与前置条件 {#goal-and-prerequisites}

将已验证的[日志规则](first-rule.md)改为精确匹配的响应。你需要当前配置以及已观察到的事件字段。先使用 `core.log` 操作；确认匹配及触发频率后，再换成其他效果。

## 匹配聊天命令 {#match-a-chat-command}

创建 `tiktok.chat` 事件，条件为 `event.data.comment` 等于（`eq`）`!hello`。记录：

```text
你好，{{ event.user.uniqueId }}
```

使用 `30` 秒的按用户冷却，防止一位观众持续触发，同时保留其他观众的独立机会。命令适合精确相等匹配；包含匹配也会接受前后带其他文字的消息。请根据意图选择。

## 记录符合条件的礼物 {#record-a-qualifying-gift}

创建 `tiktok.gift` 事件，条件为 `event.data.diamondCount` 大于或等于（`gte`）`10`。记录：

```text
收到 {{ event.user.uniqueId }} 送出的 {{ event.data.giftName }}
```

先检查真实礼物详情。礼物钻石值、重复次数和完成标志是不同字段。如果需要仅在连送完成时响应，先确认收到的载荷提供 `event.data.repeatEnd`，再使用布尔值 true 条件。仅凭名称筛选，不能推断每次连送完成只执行一次。

## 限制共享响应频率 {#bound-a-shared-reaction}

创建 `tiktok.follow` 事件，设置 `10` 秒全局冷却，并关联一个简短的日志操作。如果两位观众在该时间间隔内关注，全局冷却可能抑制后一次响应。需要每位观众分别享有触发额度时，改用按用户范围。

使用多个操作时，请排列好列表；每一步都必须执行时，保持全部操作模式。随机模式选择一个操作，不会随机排序并执行所有步骤。仅在后续步骤需要等待时添加 `core.delay`。

## 预期结果与状态 {#expected-result-and-status}

本地引擎支持这些触发器、字段筛选和冷却控件。运行应对应一个被接受的匹配事件，并受冷却和启用状态约束。提供商特有的行为仍需通过真实直播验证。

## 恢复方法 {#recovery}

配方失败时，请恢复到单个日志操作，查看载荷，并暂时简化条件。排查期间不要在繁忙直播中关闭限流。音频、积分、HTTP 和插件测试可能产生实际效果。添加这些依赖前，先检查[音频设置](../plugins/audio.md)、[变量](variables.md)和[运行诊断](../preferences/troubleshooting.md)。
