---
title: "复用与审查运行时变量"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
translationRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
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

## 单独审查 Cloud 变更 {#review-cloud-changes-separately}

宿主支持时，变量的 **更改** 控件提供经过审查的原生发送、获取、应用/采纳及撤回操作。先保存本地编辑，再选择比较项、检查其类型和值，并明确选择所需方向。获取 Cloud 候选项与将它应用到本地是两个步骤；撤回共享值也不同于删除本地行。

秘密值或被阻止的值在比较中会被遮蔽，且不能通过此审查路径发送。不要为了绕过限制而将秘密值改成普通文本。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

本地变量及按能力启用的审查式共享均可使用；本地保存从不自动上传。无效键、无效类型值和过期修订会阻止保存。重试前，请修正高亮字段或重新加载。账户故障见 [Cloud 账户](../cloud/account.md)，比较行为见[变更](../cloud/changes.md)。
