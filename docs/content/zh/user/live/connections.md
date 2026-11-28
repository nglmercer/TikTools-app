---
title: "管理直播连接"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e0d1fec65d919e7ca9946024142bbbe1bca8b3851f7e66246dcaddd101cd5215"
translationRevision: "b20f0323f197b1296421a54544439d98eb7caf7bd99ecfd23531e85c27caf521"
order: 10
---

# 管理直播连接 {#manage-live-connections}

## 目标与前置条件 {#goal-and-prerequisites}

连接指定主播，了解重新连接的行为，并单独配置插件服务器。你需要正常的[桌面安装](../installation.md)、网络访问和主播账号名。诊断操作系统问题前，请先查看[系统要求](../requirements.md)。

## 按步骤建立连接 {#connect-deliberately}

1. 打开 **连接**。输入账号名，可带或不带 `@`；显示名称并不是同一个标识符。
2. 先关闭 **使用会话 Cookie**。如果 TikTok 需要授权会话，再启用并仅在遮蔽内容的字段中输入 Cookie。Cookie 只保存在内存中，不要粘贴到诊断信息或 Cloud 设置中。
3. 直接尝试连接时，保持 **监控并自动连接** 关闭，再选择 **连接**。重试前，阅读状态和错误信息。
4. 在[直播事件流](feed.md)中检查主播是否正确，以及是否收到新事件。已连接但没有互动的直播间可能不显示活动。
5. 对同一主播使用 **重新连接**；切换主播前使用 **断开连接**。连接尝试忙碌时，**取消** 可停止该次尝试。

直接连接成功后，可启用监控以等待主播开播。等待与检查不同于已连接。编辑锁定的账号名前，请先停止监控。选择最近的主播会立即开始连接，并非仅填入输入框。

探索器会提供发现的直播间，但结果可能过时。请先刷新，再判断直播间是否仍可用。下方面板中的插件服务器连接用于配置各自的服务；插件已连接并不代表 TikTok 或 Cloud 已通过身份验证。

## 预期结果与状态 {#expected-result-and-status}

直接连接、监控及插件连接控件均可使用。重新连接同一主播会保留当前事件流上下文；切换主播会开始另一个上下文。实际能否访问 TikTok 取决于上游直播间和会话，本地测试成功并不构成保证。

## 恢复方法 {#recovery}

直播间离线时，请确认主播已经开播。会话或限流错误请遵循显示的重试条件。反复点击重新连接不会刷新身份验证。通过[故障排查](../preferences/troubleshooting.md)收集故障组件及发生时间，不要泄露凭据。

在改动真实配置之前先交互式试用:

<DemoFrame scenario="connection" locale="zh" panel="connection" title="试用 LIVE 连接面板" />

## 命令行操作 {#command-line-operations}

本指南中的每个操作也可通过 CLI 使用——离线、可编写脚本、适合代理。用法行是解析器语法,在所有语言中保持英文。

<!--@include: ../../../../.generated/cli-domain-live.zh.md-->

查看[完整命令参考](../../developer/cli.md)了解约定、JSON 输出和破坏性操作标记。

<TourFrame id="feed" locale="zh" title="交互式指南：实时 Feed" />

<TourFrame id="feedInspector" locale="zh" title="交互式指南：直播动态检查器" />

<TourFrame id="connection" locale="zh" title="交互式指南：连接创作者" />

<TourFrame id="connectionRecovery" locale="zh" title="交互式指南：连接恢复" />
