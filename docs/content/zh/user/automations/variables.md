---
title: "复用与审查运行时变量"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1c64b76271cba5b5bb23ef1f671be2bd5bf6c286afb710d2bf85ccf8cecc66b0"
translationRevision: "89c0172d7deb3ca3b7407eb44f5286b560d663f7a5d3af6814cc5fc8bb880208"
order: 40
---

# 复用与审查运行时变量 {#reuse-and-review-runtime-variables}

## 目标与前置条件 {#goal-and-prerequisites}

只修改一次共用值，就能在多个操作中复用。你需要 **自动化 → 变量**，以及支持相关变量操作的宿主。类型化编辑与经过审查的 Cloud 操作会按宿主能力显示；本地保存无需上传。

## 保存本地值 {#save-a-local-value}

1. 添加名为 `showLabel` 的变量。键以字母或下划线开头，可包含字母、数字、点、下划线和连字符，最多 `64` 个字符。避免重复键。
2. 支持类型化编辑时，选择文本并输入 `晚间直播`。其他支持的类型包括数字、布尔值、列表和秘密值。列表使用字符串 JSON 数组，例如 `["红色", "蓝色"]`。
3. 选择 **保存全局变量**。修订冲突说明已保存的集合发生变化；请重新加载，并据此审查要进行的编辑。
4. 在支持模板的操作字段中使用保存的值：

```text
{{ globals.showLabel }}：{{ event.user.uniqueId }}
```

5. 测试一个仅写日志的操作，查看渲染文本。修改 `showLabel`、再次保存，再运行新测试。现有操作定义可复用更新值，无需重新导入模板。

模板的 `params` 在应用模板时替换；`globals` 则在执行时读取。缺失的运行时键会渲染为空文本，因此用作目标或标识符前，请验证拼写。使用字段的变量建议，选择适合其触发器的事件路径。

条件还可以按匹配发布值。当过滤器声明捕获别名时，满足它的值会在该事件的操作渲染期间以 <code v-pre>{{ match.&lt;alias&gt; }}</code>（无别名时为 <code v-pre>{{ match.&lt;位置&gt; }}</code>）的形式可用。例如，对评论使用别名为 `prefix` 的“开头为”条件，可以让日志操作输出 <code v-pre>{{ match.prefix }}</code>。捕获是有上限的纯文本，仅由实际匹配的过滤器和备选生成，且不会改变冷却键。

点击模板字段旁的 **+** 可打开 **插入变量**：右侧显示精确路径，选择一行会把 <code v-pre>{{ 路径 }}</code> 追加到字段中。此事件的捕获值也在列表中，但只有在对应条件实际匹配时才会解析。要使用 <code v-pre>{{ match.&lt;alias&gt; }}</code>，请在“条件”区域设置别名（底部 **捕获** 按钮可显示别名字段）；否则使用位置形式 <code v-pre>{{ match.&lt;位置&gt; }}</code>。

## 单独审查 Cloud 变更 {#review-cloud-changes-separately}

宿主支持时，变量的 **更改** 控件提供经过审查的原生发送、获取、应用/采纳及撤回操作。先保存本地编辑，再选择比较项、检查其类型和值，并明确选择所需方向。获取 Cloud 候选项与将它应用到本地是两个步骤；撤回共享值也不同于删除本地行。

秘密值或被阻止的值在比较中会被遮蔽，且不能通过此审查路径发送。不要为了绕过限制而将秘密值改成普通文本。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

本地变量及按能力启用的审查式共享均可使用；本地保存从不自动上传。无效键、无效类型值和过期修订会阻止保存。重试前，请修正高亮字段或重新加载。账户故障见 [Cloud 账户](../cloud/account.md)，比较行为见[变更](../cloud/changes.md)。

在改动真实配置之前先交互式试用:

<DemoFrame scenario="automation" locale="zh" panel="automation" title="试用自动化工作区" />

## 命令行操作 {#command-line-operations}

本指南中的每个操作也可通过 CLI 使用——离线、可编写脚本、适合代理。用法行是解析器语法,在所有语言中保持英文。

<!--@include: ../../../../.generated/cli-domain-globals.zh.md-->

查看[完整命令参考](../../developer/cli.md)了解约定、JSON 输出和破坏性操作标记。

<TourFrame id="automationVariables" locale="zh" title="交互式指南：自动化:变量" />
