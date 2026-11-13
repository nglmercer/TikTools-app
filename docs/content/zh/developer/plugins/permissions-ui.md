---
title: "插件权限与隔离 UI"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
translationRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
order: 270
---

# 插件权限与隔离 UI {#plugin-permissions-and-isolated-ui}

## 目标与前置条件 {#goal-and-prerequisites}

公开配置和有明确意图的宿主效果，同时不将插件脚本导入主应用。使用已验证清单、匹配的插件 SDK/UI 包及私有测试宿主。先阅读[运行时信任](runtimes.md)：清单能力并不是针对原生库或进程可执行文件的操作系统沙箱。

## 声明范围有限的贡献 {#declare-a-narrow-contribution}

通过清单 JSON schema 和 UI 提示描述操作字段/设置。简单声明式页面渲染可信的宿主组件。`events.publish` 控制事件意图；宿主音频还要求 `audio.play` 能力及 `audio.output` 权限。请查阅规范的[清单 schema](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json)和 Rust 解析器，不要在文档中复制完整权限列表。

处理器需要声明其增强贡献，并返回无副作用的注释/视图/日志。领域观察者需要 `events.subscribe`，通过有界投递接收稳定封装，而非内部 Rust 事件枚举或任意传输句柄。

## 构建隔离的自定义页面 {#build-an-isolated-custom-page}

清单中带 `apiVersion:1` 和 `mode:"webview"` 的 `ui` 指向编译后的 `ui/dist/` 资源。主应用承载一个不透明来源、允许脚本的框架；可选弹出窗口使用受限原生代理。自定义脚本绝不在主 Vue 运行时中执行。

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

宿主提供插件身份，客户端不能选择其他插件 ID。代理方法仅限自身设置、操作、选项、事件以及语言/主题。页面未通过代理打开时，会明确失败。即使按钮写着测试，从页面执行操作也会产生真实效果。

## 预期结果与状态 {#expected-result-and-status}

声明式渲染、内联框架和原生弹出窗口均已有契约及测试。CORS/CSP、来源窗口绑定和原生代理允许列表共同执行边界；Shadow DOM 不承担此职责。WebKitGTK 不可配置的 IPC 属性无需删除，因为原生处理器本身已受限制。

## 恢复与验证 {#recover-and-verify}

框架空白时，先检查资源路径、CSP 和不透明来源 CORS，再考虑放宽策略。出现 `broker unavailable` 时，验证承载上下文。运行 `bun scripts/task.ts test bun plugin-ui` 和 `bun scripts/task.ts test web plugin-webview-host`。[代理包](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts)负责封装/版本行为及释放语义。
