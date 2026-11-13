---
title: "事件标准化、批处理与缺口"
docId: "developer.events"
translationKey: "developer.events"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
translationRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
order: 100
---

# 事件标准化、批处理与缺口 {#event-normalization-batches-and-gaps}

## 目标与前置条件 {#goal-and-prerequisites}

消费直播事件，避免重复记账或把展示恢复误当成执行。先了解[持久化](storage.md)，使用脱敏夹具而非真实账户抓取数据。`tiktools-tiktok` 包装固定版本的签名器/解码器依赖，导出稳定的 TikTools 值；核心不会将 protobuf 对象暴露为应用 API。

## 理解已实现的处理链 {#follow-the-implemented-pipeline}

入口在异步工作前捕获原始主播/直播间连接上下文。标准化生成聊天、礼物、点赞、社交及生命周期值。核心发布原始事件，并单独管理记账和效果工作。处理器可在 `event.intel` 下附加有界的派生证据；失败时保留原始事件并报告降级诊断。生成的自动化契约仅描述实际发出的字段。

后端重放身份包含主播、直播间、方法、类型及非零消息 ID。缺少 ID 时无法确认重放。礼物连送记账使用发送者/礼物/组身份及累计最大值，因此最终包不会重复奖励相同数量。未知价格保持为零。聚合转换可通过 `previous < threshold <= total` 支持阈值跨越，但不会替代公开的原始事件。

## 实现消费者 {#implement-a-consumer}

1. 使用 `src/automation/contracts/generated/` 中的类型，或该边界的小组件领域封装解析器。
2. 序列化时保留投递游标，合并实时与补发数据时去除重叠。
3. 有序处理批次，不要每个事件都触发一次响应式渲染。
4. 显示明确的缺口/过载信号。通过有界分页或新快照恢复展示，绝不能再次调用积分或操作来恢复。

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

[Rust 契约源码](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts)生成事件注册表、schema 及样例；不要手工复制完整事件字段表。

## 预期结果与状态 {#expected-result-and-status}

重连重叠不会造成显示投递重复，也不会重复执行受重放保护支持的记账。队列、计数器、历史和重放缓存均保持有界。这不是无限事件归档，也不是通用的恰好一次保证。容量耗尽和持久化不可用会带来明确的健康状态影响。

## 恢复与验证 {#recover-and-verify}

使用脱敏礼物/表情夹具，测试最终连送包、缺少 ID、跨直播间身份及重连缺口。检查当前 `Cargo.toml` 的签名器修订，不要复制旧可靠性报告中的哈希。未知上游方法应保持未知，直到标准化及生成契约有意加入支持。
