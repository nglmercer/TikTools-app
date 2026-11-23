---
title: "准备协作与团队访问"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "zh"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "d0c0e6ce583468a8f45ef61e17b6e9aca9cb6a974b767493b875e2c8af657ee4"
order: 30
---

# 准备协作与团队访问 {#prepare-collaboration-and-team-access}

## 目标与前提 {#goal-and-prerequisites}

在保留所有者控制权的前提下，为另一位操作员分配明确的角色。远程访问由可安装的 **Remote Control** 插件提供，而不是设置中的选项卡：需要 [Cloud 登录](account.md)、匹配的后端，并为此主机安装该插件。请从只读访问和非生产环境开始。

## 配置远程访问 {#configure-remote-access}

1. 安装并启用 `tiktools.remote-control`，然后打开其 **Remote control** 页面。
2. 创建团队，将**此电脑**注册为主机并使用易于识别的设备名称；启动中继链路，等待状态达到 `ready`。
3. 创建邀请，并通过可信渠道交付一次性令牌。邀请不等于已批准的活动会话。
4. 检查待处理请求和所请求的角色。只批准预期的操作员；在主机链路确认之前，授权保持 `pendingHost`。
5. 定期检查授权和设备。不再需要时撤销授权。

委派编辑定义、以所有者身份采用定义以及批准委派执行是不同的决定。请在具体评审中阅读保存的源、引用的操作和权限。仅有定义预览并不授权任何效果。

## 准备控制端设备 {#prepare-a-controller-device}

在控制端设备上使用其自己的账户登录，将其注册为控制端并兑换邀请令牌。所有者随后会看到待处理请求。昵称查询需要单独部署的后端端点，当前服务可能不可用。不要因查询失败就认为对方没有账户，也不要用任意 ID 替代选择。

## 预期结果与状态 {#expected-result-and-status}

双账户流程（团队、主机链路、邀请、批准、读取命令、撤销）已在本地后端验证。设置中的 Collaboration/Teams 选项卡已移除；本版本中原生 collaboration 服务没有 Web 界面。生产环境的中继/NAT 行为、昵称查询和双机 LIVE 运行未在此确认。链路就绪本身并不能证明针对你预期操作的权限已端到端生效。

## 恢复 {#recovery}

对于过期的授权，请撤销访问并在主机链路刷新后重新检查。对于未确认的命令，请在重试前检查记录的结果。结束会话时停止中继链路或撤销访问。请勿将邀请令牌和凭据写入公开的[诊断报告](../preferences/troubleshooting.md)。

## 操作细节 {#operational-details}

每条远程命令都要经过主机的能力网关（`remote.command.dispatch`、`remote.pair.list`、`remote.grant.list`、`remote.grant.revoke`、`remote.controller.ticket`）；插件不会收到 Control API 或完整的 bearer 令牌。活动传输或团队身份本身并不授予原生访问权限。Observer 偏向读取；moderator 增加部分操作控制，automationEditor 增加部分编辑能力，custom 需要明确且经过审查的策略。它们都不授予无限制的脚本或效果。当前边界请参阅[角色与确认契约](../../developer/collaboration.md#operational-details)和[传输](../../developer/transports.md)。
