---
title: "小组件 SDK、网关与叠加层边界"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
translationRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
order: 170
---

# 小组件 SDK、网关与叠加层边界 {#widget-sdk-gateway-and-overlay-boundaries}

## 目标与前置条件 {#goal-and-prerequisites}

构建在本地预览和 OBS 输出之间共享渲染的小组件。使用已安装的 Vue 工具链，并理解[事件](events.md)。本地预览不需要网关或账户凭据。实时小组件输出需要单独安装 event-gateway 插件及其限定于小组件的凭据。

## 添加模板 {#add-a-template}

1. 在 `WidgetStage` 内定义渲染器；相对于父元素设置尺寸，不要相对于浏览器视口。
2. 使用 `src/widgets/sdk/template.ts` 中的 `defineWidget`，提供 schema、新建的样例封装及 `create(search, clock)` 工厂。
3. 返回实现 `handleEnvelope`、`clear` 和 `dispose` 的控制器，以及渲染内容。使用提供的时钟，让预览可保持样例，测试可推进时间。
4. 在 `sdk/templates.ts` 注册，再添加独立入口、构建配置、网关路由允许列表及打包条目。仅注册不会公开 URL。
5. 编辑器和独立包使用同一个 `WidgetHost.vue`；本地预览不要嵌入网关 HTML。

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

文本令牌、设计默认值和可编辑字段应保留在模板 schema 中。设计序列化为 JSON，复制的小组件 URL 携带经过验证的快照；后续保存后，需向 OBS 复制更新的 URL。

## 网关协议 {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` 负责身份验证、订阅、心跳、重连和缺口报告。小组件消费规范领域封装，不直接连接 TikTok。小组件端点为 `/ws/widgets`，不同于完整外部网关客户端接口。通过共享解析器解析封装，并保留凭据边界，不要经主 WebView 传递完整网关令牌。

## 预期结果与状态 {#expected-result-and-status}

内置本地预览和小组件 SDK 已实现。Cloud 叠加层编辑和 `cloud-overlay` 源码已存在，但真实 OBS 验证、图像/实时来源覆盖及包应用仍有限制。叠加层更改是尚未合并的后续工作。即使已有排行榜小组件代码，排名与竞赛在常规应用导航中仍隐藏。

## 恢复方法 {#recovery}

本地预览空白时，先检查控制器生命周期、schema 默认值及舞台尺寸，再考虑放宽来源策略。实时输出问题需检查网关健康、小组件凭据及明确缺口。无界面 Chromium 测试不能证明 OBS 浏览器源已通过验收。[小组件 SDK](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk)及共享协议测试是当前实现参考。
