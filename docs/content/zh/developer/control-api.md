---
title: "Control API 契约与发现"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
translationRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
order: 70
---

# Control API 契约与发现 {#control-api-contracts-and-discovery}

## 目标与前置条件 {#goal-and-prerequisites}

调用宿主发布的契约，不要编造方法名或响应 schema。使用运行中的私有宿主及匹配 CLI。规范的 [Rust 注册表](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs)描述方法；注册模块提供 schema、能力、桌面要求及并发标志。选择操作前，先发现实际构建支持的内容。

## 先检查，再调用 {#inspect-then-call}

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

最后一条命令使用隔离的验证运行时，不能验证真实直播连接。本地 stdio/IPC 在线路上使用换行分隔的 JSON；远程 WSS 每帧携带一条消息，并首先要求 `collaboration.hello`。

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

响应回传相同 ID，并包含 `result` 或 `error`。错误使用字符串 `code`、消息及可选的安全 `data`；不要套用无关 JSON-RPC 服务器的数字错误码。通知使用 `method:"event"` 及 `params:{topic,data}`。通知处理应与待响应请求的解析分开。

## 保留元数据 {#preserve-metadata}

版本化调用可以携带顶层 `meta.expectedRevision`。持久化调用还需按发现结果绑定命令 ID 和宿主实例。读取领域结果时，同时读取 `meta.resourceVersions`、`meta.command` 和 `meta.hostInstanceId`。修订号是十进制字符串，不是 JavaScript 数字。前端 `src/web/shared/api/control-contracts.ts` 中的 `callWithMeta` 辅助函数会保留此封装。

## 预期结果与状态 {#expected-result-and-status}

发现到的方法受该宿主构建支持；远程权限和资源限制仍然适用。本地所有者方法不会自动成为远程方法。通用请求与类型化 SDK 包装使用相同操作路径。共享技术参考应从注册表/schema 来源派生，不应另行手工维护方法表。

## 恢复方法 {#recovery}

出现 `method_not_found` 时，比较发现结果及客户端/宿主版本。出现 `revision_conflict` 时，保留候选项并比较当前版本。传输超时后，已接受的修改可能仍在运行；支持时应查询原先保留的命令 ID。绝不要仅为绕过不确定状态而创建新 ID。继续阅读[代理](agents.md)或[远程恢复](remote-client.md)。

## 版本化方法参考 {#method-reference}

以下内容从仓库内的发现快照生成；运行中的宿主仍是权威来源。

<!--@include: ../../../.generated/control-api.zh.md-->
