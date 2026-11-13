---
title: "插件开发概览"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "8a1d746c513fdb6ecfa6624e54dd3807ed5f6b7c123fa6eaea6cca530051e49c"
order: 240
---

# 插件开发概览 {#plugin-development-overview}

## 目标与前置条件 {#goal-and-prerequisites}

选择能够满足集成需要的最小插件边界。TikTools 从内置、用户及开发根目录发现运行时包；添加包不需要把特定插件重新编译进宿主。测试效果前，使用匹配的 devkit/SDK 和私有开发配置。

## 按开发路径推进 {#follow-the-authoring-path}

1. 从[快速入门](quickstart.md)开始，创建、验证并构建隔离的包。
2. 根据原生依赖和信任选择[运行时](runtimes.md)，不要只看文件扩展名。
3. 声明[权限与 UI](permissions-ui.md)，让宿主可以审查效果并渲染配置。
4. 通过[打包与注册表](packaging-registry.md)处理完整性、发布者信任及更新策略。

底层 Rust 权威实现是 `crates/tiktools-plugin-api`，负责清单验证、协议和 UI 契约。`crates/tiktools-plugin-sdk` 提供 Rust 作者接口；`packages/plugin-sdk` 包含 TypeScript 辅助函数及[编辑器 schema](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json)。运行时加载属于 `crates/tiktools-plugin-loader`，不属于前端。

## 理解插件贡献类型 {#understand-contributions}

操作通过类型化结果请求副作用。事件源发布触发器；观察者通过有界投递接收稳定的 `{topic,data}` 领域封装。处理器在 `event.intel` 下添加无副作用的证据，并具备截止时间、确定性合并和健康隔离。声明式设置/操作让宿主渲染常规配置。自定义编译页面在自己的框架/窗口中使用受限插件代理，其脚本绝不会成为主 Vue 运行时的一部分。

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

这些是针对宿主/SDK 的检查，不是对任意外部包的验证。

## 预期结果与状态 {#expected-result-and-status}

native、process、declarative 和基于 QuickJS 的 `napi-vm` 路径均已实现。WASM 是可选运行时槽位，常规构建中没有启用通用引擎。示例集成位于独立的 [plugins-example-tiktools 仓库](https://github.com/nglmercer/plugins-example-tiktools)；旧文档中的源码名称不代表它们是随产品捆绑的功能。

## 恢复方法 {#recovery}

发现失败时，检查清单兼容性、根目录优先级、目标及入口路径。意图被拒绝时，检查声明和获授的能力，不要绕过宿主。启动结果不确定时，必须先协调确认再替换。原生/进程边界仍受操作系统级信任限制；运行时名称本身不能证明存在沙箱。
