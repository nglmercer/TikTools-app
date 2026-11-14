---
title: "首次连接直播"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "05d7feeabb75f05a83a37d6c4d33ff3cd7da4b8fd7f463ac4b6d30d3c63e7572"
order: 30
---

# 首次连接直播 {#first-live-connection}

## 目标与前置条件 {#goal-and-prerequisites}

启用自动化前，先连接一位主播，接收事件并查看详情。连接真实直播需要可正常运行的桌面安装、网络连接，以及一位当前正在直播的主播。

<DocCallout title="开播前先练习" tone="tip">

下面的示例使用真实控件和虚构数据。无需 TikTok 账户即可连接 `demo_creator`。演示成功不代表真实 TikTok 会话有效。登录 Cloud 是另一条独立连接，不能用于 TikTok 身份验证。

</DocCallout>

## 连接步骤 {#connect}

<DocSteps>

1. **选择主播。** 打开 **连接**，或点击底部状态栏的 **连接**。输入主播账号名；开头的 `@` 可省略。请使用账号标识符，不要使用显示名称。
2. **选择连接选项。** 允许访客访问时，保持 **使用会话 Cookie** 关闭。如果直播间需要你的授权会话，再启用该控件，并仅在遮蔽内容的字段中填写 Cookie。首次直接连接时，保持 **监控并自动连接** 关闭。
3. **建立连接。** 点击 **连接**，等待状态稳定。在下方示例中使用 `demo_creator`，观察连接状态指示器。

</DocSteps>

<DemoFrame scenario="connection" locale="zh" panel="connection" title="试用直播连接面板" />

## 预期结果 {#expected-result}

状态变为已连接。打开 **实时动态**，等待事件，或请对方发送一条无害的聊天消息。检查事件所属主播及时间戳，再选择事件查看详情。选择观众姓名或头像，可查看其已跟踪的资料。

<DemoFrame scenario="feed" locale="zh" panel="feed" title="查看虚构直播事件" />

直播间没有活动时，可能确实没有新事件。积分取决于已启用的积分规则，自动化结果取决于配置的操作和事件。接收到事件只是第一项检查，不会自动启用这些功能。

## 监控与重新连接 {#monitoring-and-reconnecting}

直接连接成功后，可使用 **监控并自动连接** 等待主播开播。它会分别显示等待、检查和连接中的状态。要修改锁定的主播字段，请先停止监控。重新连接同一主播会保留当前事件流；切换主播会开始另一事件流上下文。

## 恢复方法 {#recovery}

如果主播未开播，请先确认直播间正在直播再重试。若出现身份验证或限流错误，请查看 **诊断** 并遵循其中的重试条件；反复点击不能修复过期的上游会话。

<DocCallout title="会话 Cookie 应保密" tone="warning">

如果需要 Cookie，请获取新的授权会话，且不要将其值放入截图、日志或支持请求。它仅保存在内存中，并非可复用的 Cloud 凭据。测试另一位主播前，请先断开连接。

</DocCallout>
