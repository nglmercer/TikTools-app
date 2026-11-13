---
title: "应用与服务架构"
docId: "developer.architecture"
translationKey: "developer.architecture"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
translationRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
order: 30
---

# 应用与服务架构 {#application-and-service-architecture}

## 目标与前置条件 {#goal-and-prerequisites}

将改动放在拥有对应状态的服务中。先阅读[环境设置](setup.md)，并理解现有宿主与明确创建的独立宿主有何区别。桌面构建一个 `AppCore`，用一个 `ControlApi` 包装它，再向 WebView、本地 CLI 及代理公开该实例。

## 跟踪一次操作 {#trace-an-operation}

1. 在 `crates/tiktools-control-api/src/modules/` 中找到方法注册。注册提供 schema、策略及发现元数据。
2. 沿处理器进入 `crates/tiktools-core/src/control/` 及所属服务。领域验证和持久化修改应放在这里，不应放在传输适配器中。
3. 在 `crates/tiktools-client` 添加类型化包装；其对等测试会比较 SDK 覆盖与注册的接口。
4. 通过 `src/web/shared/api/control-client.ts` 使用该方法。Vue entity 管理展示状态；桌面不会把数据库或原生集成对象交给 WebView。

Winit 管理事件循环，Wry 管理 WebView，托盘集成管理托盘。`HostEmitter` 和领域事件将序列化值从核心传出。桌面通过事件循环代理转发工作；WebView 脚本求值仍在该线程执行。Tokio 运行异步宿主操作。

## 具体边界检查 {#concrete-boundary-check}

```sh
bun scripts/task.ts test web control-client
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
```

第一条命令检查前端客户端。第二条在保留锁文件的前提下直接运行 SDK 集成测试目标；Cargo 目标参数应使用 `rust` 透传，而非更窄的针对性测试语法。

## 预期结果与状态 {#expected-result-and-status}

所有客户端观察相同的权威操作和事件路径。`host --ipc`、`host --stdio` 和 `--standalone` 会有意创建无界面所有者；普通 CLI 命令附加到运行中的所有者。旧 `PageMessage`/`IpcRouter` 路径仅保留为兼容代码，不应新增消息变体。

## 恢复方法 {#recovery}

两个客户端结果不一致时，先比较宿主身份、主目录、构建功能及发现结果，再考虑添加其他 store。缺少插件是运行时目录问题，不应因此将其实现编译进核心。历史架构文档含有已退役的前端路径；有分歧时，以[当前核心](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/lib.rs)及[前端结构](frontend.md)为准。

![TikTools AppCore、传输、ControlClient 与真实 Vue 工作区](/architecture.svg)
