---
title: "性能与有界工作"
docId: "developer.performance"
translationKey: "developer.performance"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
translationRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
order: 110
---

# 性能与有界工作 {#performance-and-bounded-work}

## 目标与前置条件 {#goal-and-prerequisites}

在保留事件记账和正常关闭能力的前提下测量具体瓶颈。从固定构建、确定性输入和隔离配置开始。先阅读[事件](events.md)，避免把更快的事件流误认为更可靠的持久化。历史基准数字仅描述当时指定构建和机器的观察结果，不是当前产品保证。

## 测量正确边界 {#measure-the-right-boundary}

桌面发件箱区分允许丢失的展示与可靠响应。快照可合并；可靠通道溢出必须明确失败。直播入口、插件工作、处理器调用和自动化准入各有边界。增大一个队列可能把内存压力或延迟转移到下一阶段。

前端运行时在开发模式，或以 `VITE_TIKTOOLS_DIAGNOSTICS=1` 构建时，公开聚合诊断。快照包含已渲染行数、帧间隔百分位、控制客户端待处理调用、图片及所属定时器。特定 WebView 可能无法提供 JavaScript 堆数据。隐藏页面与宿主可见性决定展示兴趣，不影响业务记账。

## 进行针对性比较 {#run-a-focused-comparison}

1. 记录构建修订、平台、事件组合、是否启用图片、可见标签页及持续时间。
2. 使用相同夹具和采样周期建立基准。
3. 改变一个边界，再比较吞吐量、p95/p99 延迟、队列丢弃、缺口、保留内存及关闭排空行为。
4. 包含慢插件或慢消费者及饱和队列；空闲基准无法测试背压。

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

启用原生基准前检查功能要求；普通针对性单元检查无需真实 TikTok 连接。以 `crates/tiktools-core/tests/live_throughput.rs` 和桌面发件箱测试作为实现参考。

## 预期结果与状态 {#expected-result-and-status}

有界投影与准入路径已实现，并提供计数器和缺口报告。[当前有序写入器](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs)在修改内存前拒绝饱和写入，且禁止同步写入回退。旧性能报告描述的是已被替代的排序缺陷。应依据当前实现/测试评估缓存边界和故障恢复，不能从 UI 内存有界推断整体完全正确。

## 恢复方法 {#recover}

帧表现改善而记账健康恶化时，撤回边界改动，并检查写入器/准入结果。隐藏标签页的定时器增长时，按[生命周期清理](lifecycle.md)排查。报告中保留原始输入数、接受数及丢失分类。缺口恢复可修复显示状态，不能修复已被拒绝或结果不确定的效果。
