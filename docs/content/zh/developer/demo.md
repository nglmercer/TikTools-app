---
title: "浏览器演示架构与限制"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "695a2bf30a1a5bd78d0ad8e235b5f1289c913fc1ead020c882f343e8c42ab317"
translationRevision: "ffb5ff2b20d750b1a2eec3dff6370bb667b7ad5d933b3ba75a9b5e88c7bd911b"
order: 140
---

# 浏览器演示架构与限制 {#browser-demo-architecture-and-limits}

## 目标与前置条件 {#goal-and-prerequisites}

在浏览器中使用合成数据探索真实应用 UI。使用 Bun 1.4.4 及仓库锁定安装。明确的演示入口独立于 Rust、账户和运行中宿主。其运行时与浏览器验收检查属于本仓库。

## 构建与检查 {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

未提交的本地预览可使用 `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`。其清单会标记为 dirty，发布时会被拒绝。最终组装要求两个包报告相同的干净 Git SHA 及基础路径；提交后需重新构建。仅在需要本地浏览器工作时运行开发或预览服务器。`docs` 负责 VitePress，`demo` 负责浏览器应用包，`site build` 将二者组装成同一个静态网站。文档工作区将 VitePress 依赖与应用的 Vite 8 工具链分离。命令不得静默安装依赖或调用 Cargo。

## 保留应用边界 {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` 提供共享 app/Pinia/router 工厂；导入它不会挂载应用或选择传输。明确的演示引导在共享 `ControlTransport`/控制客户端边界提供 `src/web/demo/runtime/transport.ts`。纯种子夹具位于 `demo/fixtures/base-host.ts`，独立于 Playwright。`shared/lib/app-storage.ts` 和 `demo/storage.ts` 提供隔离存储适配器，确保重置不会清除原生偏好或其他演示实例。页面、小组件和功能应使用现有 entity store 及 `src/web/shared/api` 调用。不要创建第二套仿制应用、向普通原生启动注入假原生 API，或让演示请求穿透到真实宿主。

`demo/scenarios.ts` 验证 11 个场景 ID 以及允许的路由、语言和导览。`runtime/host.ts` 注册明确的方法处理器；发现列出这些处理器，`rpc-response` 回传请求 ID，不支持的效果立即返回错误。宿主填充虚构记录，每次下一个事件将固定时钟推进一个刻度。播放和下一个事件会在需要时先通过模拟 RPC 连接虚构主播，再立即投递事件。播放此后每两秒继续一次；暂停停止生成。工具栏显示播放状态和事件数。流在 300 个刻度后停止，最多保留 60 条事件流记录和 60 次运行。响应队列及存储记录另有各自限制。

重置演示仅清空当前 `tiktools.demo.<instance>.*` 前缀，并重新加载框架。重新加载重置模块单例、日志、i18n 和诊断。不同嵌入框架获得不同实例 ID。页面退出时卸载 Vue、释放 store、分离客户端/传输，并清空两个定时器。演示不会安装 `window.ipc` 桥，也不会执行任意 HTTP、shell、RCON、访客脚本或原生更新安装。其 CSP 拒绝网络连接。日志模板和筛选模拟使用纯共享函数；它们是前端契约示例，不替代 Rust 执行。

共享入口 `startTour(id, {router, capabilities, taskState})` 和导出的 `tourRegistry` 位于 `src/web/features/guided-help/index.ts`。七个版本化导览使用稳定页面目标，并为每种受支持语言提供独立目录。它们仅在请求时加载 Driver.js，将目标限制在工作区内，尊重减少动态效果偏好，恢复焦点，并在导航时取消有界等待。关闭导览绝不会完成任务或执行应用操作。选择教程会注明完整任务，准备匹配场景，并在开始前暂停播放。Driver 的关闭控件会关闭指南；工具栏报告结果。重置演示清空该实例的版本化进度。

## 预期结果与状态 {#expected-result-and-status}

演示展示场景模型支持的导航、本地编辑、事件展示及恢复状态。它不验证真实 TikTok 会话、OBS 浏览器源、Wry/原生行为、生产后端或多电脑环境。排名与竞赛保持隐藏；自动化更改仅在本地工作，经过审查的变量是真实原生产品功能，叠加层更改仍是未合并后续工作，更新器自动安装门控仍属实验性。

## 组件与 Markdown {#components-and-markdown}

`docs/examples/*.vue` 同时提供渲染目录和源码片段。小型按钮直接在 Markdown 中渲染，不使用桌面全局样式。有状态对话框、布局示例及应用页面使用 `DemoFrame`，它构建考虑基础路径且经过验证的 URL，仅在请求时加载 iframe。公开选项 `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` 可开放组件目录，而无需启用全部桌面开发工具。

## 恢复与验证 {#recovery-and-verification}

有意模拟缺口或故障后，请重置场景。如果演示联系外部服务，应停止并修复传输边界。分别检查内容、演示构建和网站组装；文档构建不能证明原生正确性。[测试](testing.md)区分模拟宿主、浏览器、Rust 及手动验收证据。
