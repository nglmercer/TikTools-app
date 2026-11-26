---
title: "CLI 命令参考"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "b45310d1a33614d65aa5a1d180d6c4fe4f1b7ba10562694c98481169d9ac5b19"
order: 75
---

# CLI 命令参考 {#cli-command-reference}

## 目标与前提 {#goal-and-prerequisites}

无需阅读 Rust 源码或解析人类可读的帮助输出，即可发现每个 `tiktools` 命令形式。需要一个匹配的已编译 `tiktools` 二进制文件；本参考完全离线，从不需要运行中的 host。权威来源是 `crates/tiktools-cli/src/commands/spec.rs` 中的命令规范注册表；每个领域模块拥有其解析器在错误中报告的用法字符串，因此本参考不会与解析器实际接受的语法产生偏差。

## 离线检查帮助 {#inspect-help-offline}

人类可读的帮助按领域对动词分组；过滤参数只显示单个领域部分。`--json` 形式打印机器可读的注册表，这是每个代理应该优先使用的方式：每个动词的用法、摘要和破坏性标记，加上全局 CLI 约定，由顶层 `schema` 字段进行版本控制。

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

用法错误只打印相关的命令形式并附带提示，而不是整页内容，因此输入错误永远不会滚动终端。退出码 2 始终表示用法错误；有关 host 解析、输出和密钥遮蔽规则，请参阅约定页脚。

## 生成的命令参考 {#generated-command-reference}

从 `docs/contracts/cli-help.json` 中签入的 CLI 帮助快照生成；二进制文件仍然是权威来源。用法行是解析器语法，在所有语言环境中保持英文；摘要和约定来自 CLI 自身的翻译目录，因此本参考不会与 `tiktools help` 以相同语言打印的内容产生偏差。标记为破坏性的动词会修改或销毁状态；脚本在执行前应进行确认。

<!--@include: ../../../.generated/cli.zh.md-->

## 操作细节 {#operational-details}

快照从使用 `--features local-web` 编译的二进制文件重新生成，以便特性门控的领域（cloud、publisher、web）保持完整。`docs check` 中的来源检查比较每个 Rust 源代码的哈希值，因此任何命令规范的更改都会导致构建失败，直到重新生成契约。正常的文档构建从不运行 Cargo。

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

在更改任何命令解析器或规范后运行重新生成，并将片段 diff 的检查作为更改的一部分。使用 `help --json` 的代理应将架构版本 1 视为加法稳定：新的动词、领域或字段可能会在没有通知的情况下出现；现有键保持其含义不变。
