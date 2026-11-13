---
title: "准备协作与团队访问"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "zh"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
translationRevision: "7bff4e49d94a8ee7e13b7a64bc8b66bec9090371bb6cefe0689ef63563424f8a"
order: 30
---

# 准备协作与团队访问 {#prepare-collaboration-and-team-access}

## 目标与前置条件 {#goal-and-prerequisites}

为另一位操作员授予明确角色，同时保留所有者控制权。你需要具备相应能力的桌面宿主，并明确选择本地/原生协作，或基于账户的团队。团队还需要[登录 Cloud](account.md)及匹配的已部署服务支持。请从只读访问和非生产环境开始。

## 配置所有者访问控制 {#configure-owner-access}

1. 打开 **设置 → 协作**。检查连接状态，并通过控件启用或配置所需监听器或网关。
2. 创建邀请前，审查允许的来源及访问矩阵。只选择操作员所需的区域、操作与资源。
3. 创建邀请，通过可信渠道传递私有链接或令牌。邀请并不等于已批准的活动会话。
4. 检查待处理请求及请求的授权。仅批准预期操作员，再确认其可见身份及连接。
5. 使用访问检查器审查后续授权变更。不再需要访问时，撤销邀请、身份或会话。

委托编辑定义、以所有者身份采纳定义，以及批准委托执行，是不同的决定。请在具体审查中阅读保存的源码、引用操作及权限。仅有定义预览并不授权执行效果。

## 准备基于账户的团队 {#prepare-an-account-based-team}

若提供 **设置 → 团队**，请创建有名称的团队并选中它。使用易识别的设备标签将 **此电脑** 注册为宿主，再使用 **开始提供服务** 并检查链路状态。前端已连接团队创建与本地宿主注册流程；这些操作不同于邀请队友。控制端设备有独立的注册角色。

选择收件人、角色及邀请有效期，然后审查收到的请求、成员和设备。昵称搜索需要单独部署的后端端点，当前服务可能不提供。搜索失败不代表对方没有账户，也不要用任意 ID 代替选择来绕过流程。

## 预期结果与状态 {#expected-result-and-status}

所有者访问控件及前端团队创建/注册功能已存在。完整远程协作流程仍属实验性：真实双电脑直播操作、生产昵称查询、NAT/中继行为，以及各平台和效果恢复路径，均未在此确立。链路就绪本身不能证明预期操作的端到端权限执行正确。

## 恢复方法 {#recovery}

授权或宿主修订过期时，重新加载并再次审查。命令未确认时，重试前先检查已记录结果。会话结束时停止服务或撤销访问。不要在公开[诊断报告](../preferences/troubleshooting.md)中包含邀请令牌或凭据。

<DemoFrame scenario="collaboration" locale="zh" title="演示：协作" />
