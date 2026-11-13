---
title: "代理与类型化 Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
translationRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
order: 80
---

# 代理与类型化 Rust SDK {#agents-and-the-typed-rust-sdk}

## 目标与前置条件 {#goal-and-prerequisites}

面向桌面使用的同一宿主编写脚本或代理。你需要运行中的隔离宿主、匹配的 `tiktools` CLI，以及[发现接口](control-api.md)访问。SDK 位于 `crates/tiktools-client`，公开直接、已连接本地及可选远程后端，不会在普通客户端路径中引入另一个业务核心。

## 从只读发现开始 {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

生成操作前，先确定支持的能力和参数 schema。不能仅凭 `system.info` 推断 TikTok 已连接、OBS 可用或允许修改。`system.doctor` 会区分运行时探测、跳过的检查与构建能力。

Rust 异步函数可附加到现有所有者：

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

调用者提供 Tokio 运行时。类型化方法委托给已注册的 RPC 方法，SDK 对等集成测试保障覆盖率。工作流程、自动化或所有者日志需要版本和命令回执时，使用 `call_with_meta` 或 `call_value_with_meta`；仅返回领域数据的辅助方法有意保持更简单的返回形式。

## 提交有明确意图的变更 {#submit-intentional-changes}

持久化操作需保留方法、规范化参数、原始命令 ID、宿主实例及发现要求的读取修订。提交前保存用户候选项。远程自动化草稿必须明确保持禁用且归作者所有；它等待所有者审查，并不代表获准执行。所有者采纳与激活具有独立的本地契约。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

代理获得与其他客户端相同的验证、事件和失败结果。代理访问已实现；权限取决于选定后端和当前授权。确认结果未知时，应查询原始结果，不能自动换新 ID 重新提交。封闭式远程摘要无法重建完整定义，因此需私下保留提交的源码。

添加方法后，运行 `bun scripts/task.ts rust test -p tiktools-client --test parity --locked`。[SDK 源码](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs)及规范注册表负责定义签名；历史代理示例可能落后于新的所有者恢复协调器。
