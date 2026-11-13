---
title: "测试与证据边界"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1872b905a3eda616e94cae8902af1ca659158e89dc8157ab9029a70fda00ce95"
translationRevision: "e1fe1f41fa9baf639efda86782687c1cb75e79fa04d7a881c72075d1d6047e6b"
order: 210
---

# 测试与证据边界 {#tests-and-evidence-boundaries}

## 目标与前置条件 {#goal-and-prerequisites}

选择能够证明所改行为的检查。使用 Bun 1.4.4 和遵循锁文件的干净安装，仅在受影响的原生边界使用 Rust。测试与生产宿主、凭据和账户隔离。浏览器夹具通过可为其边界提供有用证据，但不能证明其下每项集成都正常。

## 选择针对性套件 {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

默认快速套件组合 Bun 单元测试及 Web/小组件 Vitest 套件。针对性运行器验证参数，并拒绝缺少 Rust 测试筛选条件的情况，不会接受零匹配测试。需要超出针对性运行器语法的 Cargo 目标参数时，使用 `rust` 透传。

## 验证实际边界 {#verify-the-actual-boundary}

Vue 组件测试使用共享模拟控制客户端。主 Playwright 测试针对导航前安装的模拟 `window.ipc` 宿主，测试同一个 SPA。未知 RPC 会明确失败，并检查控制台/页面错误。它们运行 Chromium，不是 Wry。插件 UI 浏览器套件位于外部插件仓库。小组件网关套件测试共享线路协议/客户端及构建资源。

获授权的浏览器工作在构建资源后，使用 `bun scripts/task.ts test e2e`。这会启动浏览器，因此有意与文档编辑和无界面源码审查分开。原生外壳、OBS 浏览器源、手机、操作系统凭据存储、更新器替换和双电脑测试，需要真实环境及明确证据。

## 执行时间与 CI 成本 {#execution-time-and-ci-cost}

本地先使用文件/名称筛选，或 `bun scripts/task.ts test --changed`。完整前端运行仍隔离所有文件；Vitest 使用 `vmThreads` 在每个工作线程中复用 DOM 运行时，不会跨文件共享应用状态。

CI 比较完整推送或拉取请求范围。经过审查的文档路径会省略原生作业；Vue/演示变化运行前端门控。原生源码、构建基础设施、依赖清单、未知路径或无法比较的情况，会保留所有现有门控。`scripts/ci-scope.mjs` 是经过测试的选择策略。手动 CI 提供 `frontend`、`native` 或 `full`；可复用调用者默认完整验证。最终 CI 状态会拒绝失败或被取消的选定作业。

Rust 缓存按角色、操作系统、工具链、锁文件及源码修订分隔。CI 保留行表调试信息并禁用增量会话，以减少缓存体积。SQLite/配置测试在共享资源被独立隔离前，继续串行运行。不要为了获得更快计时而移除该保护。

网站工作流程为手动触发，因此编辑或推送文档不会启动第二次远程构建。发布恢复也为手动：默认只检查 Linux，不发布。发布需要同时满足平台门控和明确的发布选择。不要仅为确认 Markdown 编辑而调度或重复付费运行器工作。

## 预期结果与状态 {#expected-result-and-status}

改动报告应列出通过的套件及其范围。实现变更需运行前端 lint/类型/构建检查及必需 CI 门控；文档内容使用 `bun scripts/task.ts docs check` 和 `docs test`。不要仅为让测试命令下载更新工具而改变依赖。

## 恢复方法 {#recovery}

筛选无匹配时，修正筛选，不要报告成功。夹具缺少新增必需 RPC 时，扩展其明确契约，不要让未知调用直接成功。清晰记录不确定或跳过的手动验收。[测试运行器](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts)定义当前命令语法；[性能](performance.md)说明可复现的负载证据。

## 运行细节 {#operational-details}

安装 Bun 依赖不会安装 Playwright 浏览器。使用下方锁定的本地命令；Linux 的 `--with-deps` 还安装系统包，需要相应 OS 权限。主 `playwright.config.ts` 管理 Vite 和假宿主 SPA 测试，不驱动 Wry；真实编译插件 UI 用例还需要可选仓库的 UI 构建。Widget 浏览器测试需要自己的资源/配置。站点测试使用 `playwright.site.config.ts` 和已组装 docs/demo，应先构建再运行正式 `docs test`；后者运行文档脚本、demo/帮助、生产前端构建及 Chromium。跳过用例不算验收。

`bun run ci` 与 workflow 都用 `-- --test-threads=1` 串行运行 Rust workspace。本地 CI 不代替 MSRV/OS 矩阵。PR 根据路径选择 frontend/native/desktop/MSRV/plugin-runtime；CodeQL 仅 tag/手动触发，本仓库未配置 dependency-review workflow，外部策略可能不同。普通纯文档路径不会启动手动站点 workflow，因此应本地执行。正式 gate 使用稳定 Bun 1.4.4；VitePress 单元测试限定 fixture，`docs check` 验证整个语料。

```sh
bun x --no-install playwright install chromium
# Linux, when browser system libraries are missing:
bun x --no-install playwright install --with-deps chromium
bun scripts/task.ts test e2e
TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build
bun scripts/task.ts docs test
```
