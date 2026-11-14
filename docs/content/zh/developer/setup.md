---
title: "开发环境设置与开发循环"
docId: "developer.setup"
translationKey: "developer.setup"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3dd24191fb33ac1ebfd42ca33e68a5a97537a89df09533c4588207ea6087daa5"
translationRevision: "45d5894c226a273bc195b0e2f4970c4dacb660cf9a4b7e48255df162c2ea416e"
order: 10
---

# 开发环境设置与开发循环 {#development-setup-and-loops}

## 目标与前置条件 {#goal-and-prerequisites}

从当前 `remake` 源码构建，同时不改变用户已安装的配置。使用 `package.json` 指定的 Bun 1.4.4、Rust 1.95 或更新版本，以及兼容锁定 Vite 的 Node：`^20.19.0 || >=22.12.0`。CI 当前使用 Rust 1.99.1。源码编译与[二进制安装](../user/installation.md)是不同流程。

## 平台设置 {#platform-setup}

Windows 上安装 MSVC Build Tools、WebView2，以及供 vendored OpenSSL 使用的原生 Strawberry Perl。Cargo 包装器会发现 MSVC 环境和原生 Perl，同时保留明确设置的 `OPENSSL_SRC_PERL`/`PERL` 覆盖值。MSYS/Cygwin Perl 不适合此 MSVC 路径。LLVM 的 `lld-link` 为可选项。

Ubuntu 24.04 桌面开发的 CI 依赖包含 `libasound2-dev`、`libappindicator3-dev`、`libgtk-3-dev`、`libwebkit2gtk-4.1-dev`、`libxdo-dev` 和 `pkg-config`。AppImage 打包还需要 GStreamer 开发元数据、base/good/libav 插件及其扫描器。可选的 `mold` 可加速 Linux 链接。Android/Termux 使用原生 Android 工具链，不使用这些桌面库。

## 安装依赖 {#install-dependencies}

```sh
git switch remake
bun ci
```

使用已提交的 Bun 和 Cargo 锁文件。`bun install` 用于有意更改依赖的情况。文档是明确的独立工作区：其稳定版 VitePress 使用隔离的 Vite 依赖，应用则保留 Vite 8。不要强行将两者统一到一个 Vite 主版本。

## 选择开发循环 {#choose-a-loop}

```sh
bun run start                 # 管理 Vite 与 Wry 桌面开发宿主
bun run dev                   # 原生本地浏览器开发，不准备开发插件
bun scripts/task.ts serve     # 仅启动 Vite；原生操作需要真实宿主/适配器
bun scripts/task.ts packaged  # 使用打包后的前端路径，而非开发 URL
```

`start` 启动 Vite，等待分配的 URL，再让桌面通过 `TIKTOOLS_DEV_URL` 连接。它可从 `TIKTOOLS_PLUGINS_REPO` 或相邻目录中找到的独立 `plugins-example-tiktools` 检出准备插件，不会编译应用仓库内的插件实现。设置 `TIKTOOLS_SKIP_DEV_PLUGINS=1` 可跳过准备。不会自动替换宿主；已有桌面所有者会导致启动错误。

启动前，请先安装外部插件仓库自己的依赖。在 TikTools 仓库中执行以下命令，使用锁文件安装工具链、在不启动宿主的情况下准备插件，并在 `dist/plugins/` 中生成可分发归档。准备过程会重新构建插件界面，并替换 `.dev-plugins` 中的生成内容。`TIKTOOLS_PLUGINS_REPO=off` 会清空该暂存目录，不加载外部开发插件；用户已安装的插件另行管理。

下面的相邻目录路径只是本地示例。CI 不需要该插件仓库；目录布局不同时，可将 `TIKTOOLS_PLUGINS_REPO` 设为绝对路径。

```sh
bun install --frozen-lockfile --cwd ../plugins-example-tiktools
bun scripts/task.ts prepare-plugins
bun scripts/task.ts plugins --all
```

进行隔离的原生实验时，启动宿主前将 `TIKTOOLS_HOME` 设为可丢弃目录。控制该宿主的所有 CLI 终端使用同一主目录。绝不要删除运行中宿主的所有权锁。

## 验证与恢复 {#validate-and-recover}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

迭代时运行针对性测试，合并前保留全部必需 CI 门控。对会调用 Cargo 的签名测试计时时，不要同时对同一 target 目录运行多个 Cargo 构建。启动因旧宿主而被拒绝时，正常停止所有者进程；缺少链接器或 WebView 依赖时，先修复平台环境，再修改应用逻辑。
