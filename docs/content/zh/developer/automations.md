---
title: "自动化执行与脚本契约"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
translationRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
order: 150
---

# 自动化执行与脚本契约 {#automation-execution-and-script-contracts}

## 目标与前置条件 {#goal-and-prerequisites}

使用宿主描述符及生成的事件契约实现自动化。在私有配置中工作，有意测试前关闭外部效果。先理解[事件标准化](events.md)和[控制元数据](control-api.md)；Vue 编辑器配置的是由 Rust 负责的执行过程。

## 构建最小规则 {#build-a-minimal-rule}

行为操作保存一个已配置的操作类型。行为事件选择触发器、筛选、冷却和操作 ID。工作流程图则组合版本化节点和端口。内置描述符来自核心，插件描述符来自已安装清单。不要在编辑器中硬编码第二份操作目录。

1. 从 `src/automation/contracts/generated/event-registry.generated.ts` 选择已发出的触发器。
2. 使用其样例选择真实点分字段，例如 `event.user.uniqueId` 或 `event.data.giftName`。
3. 从日志或禁用的操作开始，再使用脱敏事件测试筛选和冷却。
4. 通过所属原生协调器保存，并在需要时保留编辑器候选项和已审查版本。

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

[生成的契约目录](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated)负责前端事件类型、注册元数据及样例。公开额外字段时，同时扩展 Rust 契约并重新生成。

## 脚本边界 {#script-boundary}

`core.code` 和工作流程脚本节点运行受限的 QuickJS。它们接收 JSON `event`、`inputs` 和 `data`（inputs 的别名）。没有 Node 加载器、文件系统、网络、WebView 或数据库对象。转换应保持确定性并可序列化为 JSON；HTTP、积分、音频或插件效果应通过已配置的宿主操作实现。源码/结果大小限制和执行预算由宿主负责，不由浏览器超时负责。

## 预期结果与状态 {#expected-result-and-status}

保存的规则通过核心运行时及其能力代理执行。处理器可添加 `event.intel` 证据，而不替换原始字段。自动化更改是本地审查入口，不是账户范围的配置同步。经过审查的变量已有真实原生实现；叠加层更改是另一项尚未合并的后续工作。

## 恢复方法 {#recovery}

冲突时比较当前记录，同时保留候选项。远程草稿保持禁用，等待所有者审查和权限转移；仅检查预览/清单不授权执行。外部效果结果未知时，需使用保留的效果检查/恢复流程。重放 UI 历史绝不能再次触发自动化。受控规则库导入见[模板](templates.md)，边界专属证据见[测试](testing.md)。
