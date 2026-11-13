---
title: "为 OBS 准备叠加层"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6062b2d3d3248eaa9ab1c8f73082c8d4a6bbe81d199f8a45b6c8c28dea617d46"
translationRevision: "68fa10cf6edcd14da890258c4bcdaad2e0ffce5419d18e06d9ee3296a4e541f6"
order: 10
---

# 为 OBS 准备叠加层 {#prepare-overlays-for-obs}

## 目标与前置条件 {#goal-and-prerequisites}

设计易读的小组件，并将其本地浏览器源连接到 OBS。你需要运行 TikTools，具备叠加层网关能力，并在同一台电脑上运行 OBS，才能使用默认回环地址。实时输出还需要对应的传入事件或已配置的排行榜来源。

## 设计与预览 {#design-and-preview}

1. 打开 **叠加层** 并查看网关状态。选择关注、礼物、聊天、分享、订阅、点赞或排行榜。
2. 在设计检查器中编辑图层、可见文本、颜色、间距及头像选项。针对实际直播分辨率使用足够的对比度及易读字号。
3. 重播预览以检查布局。预览内容用于评估设计，不能证明直播数据已送达。
4. 使用撤销/重做控件处理本地编辑，再点击 **保存设计**。切换小组件类型时，如有未保存的变更，会提示是否放弃。
5. 在预览/源面板中复制源 URL。显示的 URL 会遮蔽凭据；请使用复制控件获取可用地址。该 URL 应保密。
6. 添加 OBS 浏览器源并粘贴复制的 URL。保持 TikTools 运行，正式依赖该源直播前，先验证一个相关事件。

默认网关为 `127.0.0.1:17452`。回环 URL 指向运行浏览器源的电脑，因此将其复制到另一台电脑不会自动访问此宿主。排行榜小组件还需选定排行榜 ID；请参阅[排行榜状态](../points/rankings.md)。

## 使用 Cloud 库 {#use-the-cloud-library}

宿主提供 Cloud 叠加层能力时，可从叠加层打开库。浏览本地设计或 **我的叠加层**，用编辑器创建 Cloud 叠加层，或打开已有 ID 进行修订。Cloud 编辑器支持文本、实时字段、图像及画布尺寸；图片上传使用单独流程。本地设计保存与 Cloud 创建/修订是不同操作。

## 预期结果与状态 {#expected-result-and-status}

本地设计、网关控件，以及按能力开放的 Cloud 编辑器/库流程均可使用。此版本不提供共享叠加层 **更改** 适配器。OBS、真实直播及各操作系统仍需在你的环境中验收。

## 恢复方法 {#recovery}

缺少输出时，请检查网关状态、资源、凭据、源 URL 及所选事件类型。Cloud 修订冲突时，先重新加载当前修订再编辑。不要在截图中公开源令牌；通过[诊断](../preferences/troubleshooting.md)报告网关错误。

<DemoFrame scenario="overlays" locale="zh" tour="overlay" title="演示：叠加层" />

## 运行细节 {#operational-details}

添加实时 OBS 来源前，先单独安装 Event Gateway 插件，启用并确认运行。状态对应恢复：missing 安装；disabled 启用并检查可用性；stopped 启动；credentialUnavailable 重启 gateway 一次；assetsMissing 修复/重建归档内 widget 资源；starting 稍后重试；unreachable 检查 loopback 端口及服务。使用生成来源 URL 中的 widget 专属凭据。ready 或本地预览不证明真实 LIVE 事件或 OBS 接收，请在目标来源进行无害测试。
