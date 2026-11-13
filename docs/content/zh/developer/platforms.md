---
title: "平台与原生 Termux 构建"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
translationRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
order: 20
---

# 平台与原生 Termux 构建 {#platforms-and-native-termux-builds}

## 目标与前置条件 {#goal-and-prerequisites}

选择匹配目标操作系统的构建。桌面开发需要 Bun 1.4.4、Rust 1.95 或更新版本，以及[环境设置](setup.md)中说明的平台 WebView 依赖。Termux 在 Android 私有主目录中直接构建 Rust CLI；Android 共享存储不适合可执行文件和 Unix 套接字。

## 构建适合的宿主 {#build-the-appropriate-host}

Windows 使用 MSVC Build Tools 和 WebView2。原生 Strawberry Perl 支持 vendored OpenSSL；不要用 MSYS Perl 代替。Linux 桌面需要 GTK3、WebKitGTK 4.1、音频开发库和 `pkg-config`。Winit/Wry 集成属于 `crates/tiktools-desktop/src/platform.rs`；核心业务服务保持独立于窗口工具包。

ARM64 手机上，先验证 `rustc -vV` 报告 `aarch64-linux-android`。在 Termux 中运行：

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

此处有意直接使用 Cargo：手机原生流程不需要 Bun 或 Node。在支持 Bun 的桌面上，通过 `bun scripts/task.ts rust ...` 调用 Cargo，以保留链接器/缓存处理。使用两个手机终端时，先启动 `./target/debug/tiktools host --ipc`，后续 CLI 调用使用相同的 `TIKTOOLS_HOME`。

## 预期结果与状态 {#expected-result-and-status}

本地 IPC、SQLite 和可移植 process/QuickJS 插件均已实现 Android 路径。进程产物必须面向 Android/Bionic。Android 不支持原生库和 `.node` 扩展。桌面音频、热键、通知、文件选择器和 WebView 没有手机后端。持久化远程配置会明确失败，因为 Android 尚未选定凭据存储后端；SDK 凭据可保留在内存中。

## 验证与恢复 {#verify-and-recover}

在检出目录根运行 `bash scripts/termux/doctor.sh`、`bash scripts/termux/test.sh` 和 `bash scripts/termux/smoke.sh`。如跳过前端检查，应如实报告。`protoc` C++ 符号损坏时，先比较 `env -u LD_LIBRARY_PATH protoc --version`，再修改库。缺少 CA 证书包必须修复，不能绕过。仅交叉编译不能证明手机端通过验收。[Termux 冒烟脚本](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh)记录了针对性探测与剩余限制。
