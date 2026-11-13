---
title: "提交范围明确的贡献"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
translationRevision: "69b624eecbaba67197d3daa99516e6f19328d612ebec2e115f69d9401430739c"
order: 230
---

# 提交范围明确的贡献 {#contributing-focused-changes}

## 目标与前置条件 {#goal-and-prerequisites}

准备其他贡献者可以审查和验证的改动。编辑前，阅读仓库的 [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md)、适用的 `AGENTS.md` 及[环境设置](setup.md)。使用 Bun 1.4.4，必要时使用 Rust 1.95 或更新版本、已提交锁文件，以及基于预期目标的分支。

## 定义并实现改动 {#define-and-implement-the-change}

1. 检查 `git status`，保留他人负责的工作。补丁中不要包含生成资源、凭据和本地数据库。
2. 遵循当前源码职责：服务行为属于 Rust 核心，契约属于 Control API 模块，SDK 覆盖属于类型化客户端，展示遵循[前端分层图](frontend.md)。
3. 契约变化时，同时更新规范定义并重新生成派生输出。不要手工维护另一份完整 RPC 或事件 schema 表。
4. 插件专属实现保留在各自外部插件包；通用宿主代码应发现清单，而非导入特定插件。
5. 说明具体触发条件及最终行为，包括尚未完成或实验性的路径。

## 按影响范围验证 {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

原生实现变化还需要适用的 Rust 格式、lint、测试及必需 CI 门控。仅文档改动应使用文档检查，并审查命令和源码链接，不应声称通过原生运行时验收。项目任务运行器负责支持的入口；特定原生目标参数使用 Cargo 透传。

## 预期结果与状态 {#expected-result-and-status}

审查应包含范围明确的源码变更、有意义的验证证据和如实说明的限制。仓库策略面向合入 `remake` 的拉取请求；发布标签由维护者控制。贡献说明已提供，但分支保护和远程检查配置是管理员职责，并非 Markdown 页面能保证。

## 恢复方法 {#recover}

缺少依赖或平台库导致验证失败时，应报告并修复环境，不要削弱检查。生成文件偏离时，从其权威来源重新生成并检查差异。绝不要发布会话 Cookie、密钥、凭据或未脱敏事件抓取数据。源码与旧设计不一致时，说明差异，并以当前实现编写公开指南。不要覆盖其他所有者尚未完成的文件，只为让大范围清理看起来一致。

## 运行细节 {#operational-details}

审核标记含义分开：`sourceRevision` 是当前英文正文哈希，`translationRevision` 是本篇本地化正文哈希（移除 frontmatter 并去除首尾空白）。审核源文及全部译文后，运行 `bun docs/scripts/record-review.ts developer/example.md`。随后修改任一正文会使相应标记失效。这只能检测漂移，不能证明审核者身份或语义一致；应明确比较命令、RPC/环境标识符、payload 键、限制、启用/恢复步骤及否定表达。完整翻译说明并保留技术指令。

拆分迁移时，在映射中记录每个原主题/标题及具体替代目标，指向特定章节时包含片段。检查器验证文件、片段及历史原始字节；主题覆盖仍须编辑审核。不要修改历史原文及 provenance 来让验证通过。
