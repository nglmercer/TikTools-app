---
title: "常见问题"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
order: 40
---

# 常见问题 {#frequently-asked-questions}

## 目标与前置条件 {#goal-and-prerequisites}

修改数据或凭据前，先解决常见工作流程疑问。下面的检查需要运行中的桌面应用；支持的安装包见[系统要求](../requirements.md)。除非明确说明实验性或服务依赖限制，此处讨论的本地功能均可使用。

## 登录 Cloud 会连接 TikTok 吗？ {#does-cloud-sign-in-connect-tiktok}

不会。请打开 [连接](../live/connections.md)，选择主播并单独启动直播连接。Cloud 身份用于基于账户的服务。TikTok Cookie 只能填写在指定的遮蔽连接字段中。

## 为什么连接成功后事件流仍为空？ {#why-is-a-connected-feed-empty}

安静的直播间可能没有新事件。选择全部事件、清空搜索并恢复滚动，再检查连接状态。暂停滚动不会停止接收或自动化。清空事件流不会重置积分或已持久化的统计。

## 统计仅覆盖当前会话吗？ {#are-statistics-only-for-this-session}

当前[统计页面](../points/statistics.md)查询所选主播及日期范围的持久化聚合数据。扩大日期并刷新，可查看已记录的历史。它不是完整聊天记录。积分余额使用不同规则，可能包含手动或自动化调整。

## 模板、变量与排行榜在哪里？ {#where-are-templates-variables-and-rankings}

模板和配置位于 [自动化](../automations/templates.md)；插件探索请求的是插件。运行时值位于自动化变量。实验性排名与竞赛工作区仍有 `#/rankings` 路由，但不在主导航及启动恢复中显示。常规积分榜与事件流排名面板是独立视图。

## 保存会同步所有内容吗？ {#does-saving-synchronize-everything}

不会。事件/操作更改比较本地参考。变量保存保持在本地，支持时另有经过审查的发送、采纳或撤回操作。秘密值比较会遮蔽内容。Cloud 叠加层使用创建、修订和库操作；此版本没有共享叠加层更改适配器。选择方向前请阅读[变更](../cloud/changes.md)。

## 测试是否无害？导入后会立即运行吗？ {#are-tests-harmless-and-do-imports-run-immediately}

测试可能执行实际效果，包括音频、网络请求和积分变更。请从 [core.log](../automations/first-rule.md) 开始。导入定义会保持禁用；请审查后明确激活。安装插件后，也需完成服务配置才能使用操作。

## 预期结果与恢复 {#expected-result-and-recovery}

这些检查应能定位负责的页面，无需重置。自动检查更新是可选项；自动安装仍属实验性，并被当前发布门控禁用。完整远程协作及生产昵称搜索仍依赖部署与验收。如果仍有疑问，请通过[备份](backups.md)保存数据，并收集包含版本、时间和复现步骤的[诊断报告](troubleshooting.md)。
