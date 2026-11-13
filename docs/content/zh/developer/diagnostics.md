---
title: "诊断与安全证据"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
translationRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
order: 130
---

# 诊断与安全证据 {#diagnostics-and-safe-evidence}

## 目标与前置条件 {#goal-and-prerequisites}

收集足够证据定位故障，同时避免保留账户秘密或将诊断误当成恢复。使用私有测试宿主，并了解选中了哪种[传输](transports.md)。应用安装一个会话收集器；状态组件导航到该收集器，不在渲染时追加条目。

## 检查故障 {#inspect-a-failure}

1. 打开失败连接、插件、同步轮次或更新器的上下文状态操作。键盘聚焦与悬停一样，会显示桌面日志操作。
2. 按来源、严重程度和文本筛选，再查看选中条目的上下文及重复次数。
3. 选中或筛选的条目能说明问题时，复制它们。分享前检查导出内容。
4. 将原生健康状态与前端观察进行比较：

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

直播未连接时，探测可能被跳过。已编译的能力不代表运行时可用。桌面 UI 面板为非模态；窄屏布局使用带返回位置的 `/debug`。原生文本导出使用 `system.saveFile`，浏览器则在本地下载。

## 理解保留规则 {#understand-retention}

`src/web/shared/diagnostics/log.ts` 将历史限制为 2,000 条及 2 MiB 的序列化 UTF-8。五秒内连续匹配的来源/实体事件会合并并记录重复次数。脱敏在保留前发生，包括已识别凭据及 URL 秘密位置；任意未标记的第三方秘密并不总能识别。不要向收集器添加原始设置、Cookie、RPC 参数或 HTTP 正文。

## 预期结果与状态 {#expected-result-and-status}

查看器提供有界会话历史与上下文导航，不是持久审计日志。重新加载或卸载会清空内存。清空历史不会清除连接错误或修复原因。Cloud 请求 ID 可帮助关联安全失败封装；来源覆盖取决于已安装收集器，不保证捕获每个注册表或原生错误。

## 验证与恢复 {#verify-and-recover}

运行 `bun scripts/task.ts test web diagnostics`，并查看[收集器源码](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics)。使用合成值复现脱敏、淘汰及重复事件行为。遇到冲突、未知命令或凭据过期时，返回所属恢复流程。复制诊断条目只是收集证据，绝不会重试操作或批准新信任策略。
