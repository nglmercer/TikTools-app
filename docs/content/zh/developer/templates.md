---
title: "规则模板、配置与所有者审查"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "4425e836fb6a4a8d6de9f7bf82aa8d21f268243dc9c6136de1dbf93375b9e486"
order: 160
---

# 规则模板、配置与所有者审查 {#rule-templates-profiles-and-owner-review}

## 目标与前置条件 {#goal-and-prerequisites}

导入可复用规则，同时保留本地所有权和可恢复性。使用私有宿主、必需且已安装的操作提供者，以及有效的 `.tiktemplate.json` 或 `.tikprofile.json`。启用导入规则前，先阅读[自动化](automations.md)。规则库导入属于配置工作，不能证明外部集成可访问。

## 准备文档 {#prepare-a-document}

v1 模板包含 `templateVersion`、可移植 ID、参数 schema、操作及一个关联事件。配置包含 `profileVersion`、共用参数和内联模板条目。权威解析器位于 `crates/tiktools-core/src/services/templates/` 及 `src/web/entities/rule-template/lib/`；`tests/fixtures/templates/` 中的夹具保持两端一致。

参数依次从 schema 默认值、配置、条目以及明确的对话框/CLI 值合并。完整的 <code v-pre>{{ params.* }}</code> 片段保留 JSON 值类型，嵌入片段则转为文本。事件替换留到执行时处理。运行时全局值稍后解析，不应将账户秘密固化进可分享文档。

## 预览与导入 {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

这些示例假设文件已准备好且匹配宿主正在运行。UI 中，规则模板和配置导入先暂存候选项供审查。原生 `automation.library.*` 协调器负责原子规则库转换和原始结果恢复；其当前 schema 可通过 [Control API](control-api.md) 发现。

## 预期结果与状态 {#expected-result-and-status}

经过审查的规则库导入会创建一致的关联记录，并保留所有者日志。Cloud 目录模板导入保存文档，配置导入及后续应用必须审核 enabled，不能假定禁用。缺失或禁用的操作提供者会导致需求错误，不会被静默替换。一个选中的配置包控制对应的展示上下文；配置选择并不代表远程权限。

## 恢复与验证 {#recover-and-verify}

修订冲突时，保留候选项、重新加载规则库快照，并审查实际差异。创建新的导入意图前，先解决待处理原生日志。不要将失败的更新隐式转为新建。运行 `bun scripts/task.ts test web rule-templates` 和 `bun scripts/task.ts test web library-journal` 获得针对性前端覆盖。[原生模板服务](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs)及生成的 RPC schema 优先于旧 `src/web/views/behavior` 引用。

## 运行细节 {#operational-details}

将第一个 JSON 保存为 `docs-log.tiktemplate.json`；第二个是完整的单条目 `docs-pack.tikprofile.json`。两者都明确禁用动作/事件。参数优先级为 schema 默认值 → 配置 → 条目 → CLI/对话框显式值。完整参数占位保留类型，嵌入占位转换为字符串。省略 enabled 默认 true，应使用 `--disabled` 并检查 UI 最终比较。

当前限制：每模板 8 个动作、12 个筛选，每配置 32 个模板；UI 源文本每模板 64 KiB、每配置 256 KiB；展开深度 64、65,536 节点、每记录 256 KiB、总计 8 MiB。预算错误应通过减少内容解决，不能绕过验证。目录、源文本和展开预算彼此独立。结果不确定时，再次导入前遵循[日志恢复](automations.md#operational-details)。

```json
{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false,"filters":[],"cooldownMs":1000,"cooldownScope":"global","runMode":"all"}}
```

```json
{"profileVersion":1,"id":"docs-pack","name":"Docs pack","description":"","params":{"message":"profile"},"templates":[{"params":{"message":"entry"},"template":{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false}}}]}
```

```sh
tiktools template import docs-log.tiktemplate.json --dry-run
tiktools template import docs-log.tiktemplate.json --disabled
tiktools template profile-import docs-pack.tikprofile.json --disabled
```
