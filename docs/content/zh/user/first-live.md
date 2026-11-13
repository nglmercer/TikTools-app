---
title: "首次连接直播"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb7101f2010cd57478f9f6b2ea9cbb8c0a6401975fe4b29b311c41b13c1a5185"
translationRevision: "cc3c38f169338fa2f6f33c3225a8cfa058d93611eb0485bad58586fd4d1f34d9"
order: 30
---

# 首次连接直播 {#first-live-connection}

## 目标与前置条件 {#goal-and-prerequisites}

启用自动化前，先接收一位主播的事件。你需要可正常运行的桌面安装、网络连接，以及一位当前正在直播的主播。登录 Cloud 是另一条独立连接，不能用于 TikTok 身份验证。

## 连接步骤 {#connect}

1. 打开 **连接**，或点击底部状态栏的 **连接** 操作。
2. 输入主播的账号名，开头的 `@` 可省略。请核对拼写，不要使用显示名称代替账号名。
3. 允许访客访问时，先保持 **使用会话 Cookie** 关闭。如果直播间需要你的授权会话，再启用该控件，并仅在遮蔽内容的字段中填写 Cookie。
4. 首次直接连接时，保持监控关闭。点击 **连接**，等待连接状态稳定。
5. 打开 **实时动态**。请对方发送一条无害的聊天消息，或等待真实事件到来。检查事件所属主播及时间戳。
6. 选择一个事件查看详情。选择观众姓名或头像可查看其已跟踪的资料。

## 预期结果 {#expected-result}

状态变为已连接，收到的事件会出现在事件流中。直播间没有活动时，可能确实没有新事件。积分取决于已启用的积分规则，自动化结果取决于配置的操作和事件。

## 监控与重新连接 {#monitoring-and-reconnecting}

直接连接成功后，可使用 **监控并自动连接** 等待主播开播。它会分别显示等待、检查和连接中的状态。要修改锁定的主播字段，请先停止监控。重新连接同一主播会保留当前事件流；切换主播会开始另一事件流上下文。

## 恢复方法 {#recovery}

如果主播未开播，请先确认直播间正在直播再重试。若出现身份验证或限流错误，请查看 **诊断** 并遵循其中的重试条件；反复点击不能修复过期的上游会话。如果需要 Cookie，请获取新的授权会话，且不要将其值放入截图、日志或支持请求。它仅保存在内存中，并非可复用的 Cloud 凭据。

测试另一位主播前，请先断开连接。演示可以通过虚构事件帮助你学习这些控件，但演示成功不代表 TikTok 会话有效。

<DemoFrame scenario="connection" locale="zh" tour="connection" title="演示：连接直播" />
