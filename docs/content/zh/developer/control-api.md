---
title: "Control API 契约与发现"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "b88db92d8b180d0f2e40032f600c7431910f093c73b2a32c0b30b72d7f66e625"
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

## 运行细节 {#operational-details}

使用可丢弃的 `TIKTOOLS_HOME`，按[存储指南](storage.md)清除继承的路径覆盖，启动 `tiktools host --ipc`，并在控制终端使用同一 home。以下流程假设已存在试验 workflow `docs-example`；请先创建[自动化](automations.md#operational-details)中的禁用示例。保留读取返回的 `meta.resourceVersions` 修订及 `meta.hostInstanceId`，替换 `REVISION`/`HOST_INSTANCE`。固定命令 ID 及其确切 payload。超时后查询原始 ID，再刷新资源；`accepted`、`running`、`unknown` 不允许新发副作用。修订冲突要求重新读取和审核，不能自动覆盖。`points.adjust` 为加法操作：接受命令 ID，不接受预期修订。

`--include-meta` 保留结果及元数据，默认输出只提取结果。`--dry-run --explain` 验证/规划但不执行；说明写入 stderr，JSON 结果写入 stdout。破坏性调用要求 `--confirm`。默认输出隐藏已识别秘密；`--secrets-visible` 只改变 CLI 呈现，不能恢复宿主已经隐藏的值，仍需人工检查。`api events` 按超时/数量限制并筛选主题。运行时 discovery 决定功能与权限；下方生成的契约描述保留注册表原文。

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
