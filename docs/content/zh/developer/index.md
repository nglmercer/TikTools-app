---
title: "开发 TikTools"
docId: "developer.overview"
translationKey: "developer.overview"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1ee65a70172f8154eeaf60fb1a3069192769582a4ef264426d0aefaf346e8003"
translationRevision: "7f8b04a6e385380aa94a58ef10fb75e21b76719459476fcc038665d07d1441e8"
order: 0
---

# 开发 TikTools {#develop-tiktools}

TikTools 在类型化控制操作背后维护单个原生 AppCore。Vue 渲染桌面、本地浏览器和控制端界面；Winit/Wry 管理桌面窗口。CLI 与 SDK 使用相同操作。先阅读[开发环境设置](setup.md)，再选择能够覆盖改动的最小开发循环。

## 选择入口 {#choose-an-entry-point}

| 工作 | 入口 | 验证范围 |
| --- | --- | --- |
| Vue 组件与 store | Vite、Vitest 及注入的 ControlClient | 界面行为；不验证原生窗口。 |
| 桌面集成 | `bun run start` | 开发 WebView 与 IPC 集成。 |
| 原生服务 | 针对性 Cargo 测试 | 核心契约、持久化及服务行为。 |
| 无界面浏览器访问 | `bun run dev` | 原生宿主、本地适配器及带配对的 Vite。 |
| 文档 | `bun scripts/task.ts docs dev` | Markdown 及隔离的 VitePress 构建。 |
| 公开演示 | 独立演示入口及模拟传输 | 虚构且确定的场景，不验证上游服务。 |

## 源码职责 {#source-ownership}

- `crates/tiktools-core`：服务、操作和 SQLite，不依赖 WebView。
- `crates/tiktools-control-api`：注册表、JSON-RPC 路由、IPC 及发现。
- `crates/tiktools-desktop`：Winit/Wry、UI 线程桥接、生命周期及打包资源。
- `src/web/app`：引导、路由及外壳。`pages` 组合 `widgets` 和 `features`；`entities` 管理领域 store/契约；`shared` 包含可复用 UI 和基础设施。
- `src/automation/contracts/generated`：从 Rust 生成的版本化前端事件契约。不要在演示或指南中重新手写这些 schema。
- `packages`：现有 TypeScript 插件 SDK/UI 包。插件实现位于外部插件仓库。

## 贡献边界 {#contribution-boundary}

保持原生、本地浏览器、远程控制端及演示入口明确。真实入口缺少原生宿主时应报错，不能静默选择模拟对象。监听器、订阅和定时器应随所属作用域释放。保持 Rust/Bun 锁文件和必需 CI 门控完整。

用户安装请参阅[用户指南](../user/installation.md)。这些开发检查本身不能证明真实 TikTok 连接、OBS 渲染、原生 WebView 兼容性或双电脑协作正常。
