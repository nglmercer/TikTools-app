---
title: "为 OBS 准备叠加层"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
translationRevision: "6e039c4e954bc3921ce6b837ace0ff58f1e3243802117ed9b2332a30c7cc3197"
order: 10
---

# 为 OBS 准备叠加层 {#prepare-overlays-for-obs}

## 目标与前置条件 {#goal-and-prerequisites}

设计易读的小组件，检查预览，并将浏览器源连接到 OBS。你需要运行 TikTools，具备叠加层网关能力，并在同一台电脑上运行 OBS，才能使用默认回环地址。实时输出还需要对应的传入事件或已配置的排行榜来源。

<DocCallout title="先预览设计" tone="tip">

配置直播前，先用示例编辑器熟悉控件。虚构预览用于评估外观，不能证明真实直播数据已送达或 OBS 已接收。

</DocCallout>

## 设计与预览 {#design-and-preview}

<DocSteps>

1. **选择小组件。** 打开 **叠加层**，查看网关状态，再选择关注、礼物、聊天、分享、订阅、点赞或排行榜。
2. **保证清晰易读。** 在设计检查器中编辑图层、可见文本、颜色、间距及头像选项。针对实际直播分辨率检查对比度和字号。
3. **检查预览。** 重播预览以检查布局。使用撤销/重做控件处理本地编辑，再点击 **保存设计**。切换小组件类型时，如有未保存的变更，会提示是否放弃。

</DocSteps>

<DemoFrame scenario="overlays" locale="zh" panel="overlays" title="试用叠加层设计控件" />

<DocSteps>

1. **复制源地址。** 在预览/源面板中，使用复制控件获取可用的源 URL。显示的 URL 会遮蔽凭据；复制得到的 URL 应保密。
2. **添加到 OBS。** 添加 OBS 浏览器源并粘贴复制的 URL。保持 TikTools 运行，正式依赖该源直播前，先验证一个相关事件。

</DocSteps>

默认网关为 `127.0.0.1:17452`。回环 URL 指向运行浏览器源的电脑，因此将其复制到另一台电脑不会自动访问此宿主。排行榜小组件还需选定排行榜 ID；请参阅[排行榜状态](../points/rankings.md)。

## 使用 Cloud 库 {#use-the-cloud-library}

宿主提供 Cloud 叠加层能力时，可从叠加层打开库。浏览本地设计或 **我的叠加层**，用编辑器创建 Cloud 叠加层，或打开已有 ID 进行修订。Cloud 编辑器支持文本、实时字段、图像及画布尺寸；图片上传使用单独流程。本地设计保存与 Cloud 创建/修订是不同操作。

## 预期结果与状态 {#expected-result-and-status}

本地设计、网关控件，以及按能力开放的 Cloud 编辑器/库流程均可使用。此版本不提供共享叠加层 **更改** 适配器。请在目标操作系统中检查 OBS 设计，并确认真实直播输出。

## 恢复方法 {#recovery}

缺少输出时，请检查网关状态、资源、凭据、源 URL 及所选事件类型。Cloud 修订冲突时，先重新加载当前修订再编辑。通过[诊断](../preferences/troubleshooting.md)报告网关错误。

<DocCallout title="源 URL 含有凭据" tone="warning">

不要在截图或支持请求中公开令牌。使用生成源 URL 中的小组件专属凭据，并在目标 OBS 来源中用无害事件验证接收情况。

</DocCallout>

## 运行细节 {#operational-details}

添加实时 OBS 来源前，先单独安装 Event Gateway 插件，启用并确认运行。各状态对应的恢复方法：missing → 安装；disabled → 启用并检查可用性；stopped → 启动；credentialUnavailable → 重启网关一次；assetsMissing → 修复或重建打包的小组件资源；starting → 稍后重试；unreachable → 检查回环端口及服务。ready 状态或本地预览都不能证明真实直播事件已送达或 OBS 已接收。
