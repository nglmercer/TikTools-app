---
title: "Termux 与无界面运行的适用范围"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "zh"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
translationRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
order: 50
---

# Termux 与无界面运行的适用范围 {#termux-and-headless-scope}

## 目标与前置条件 {#goal-and-prerequisites}

了解 TikTools 的哪些部分可以在没有桌面 WebView 的情况下运行。这是高级工作流程，并非 Windows/Linux 安装器的 Android 版本。项目提供面向 Termux 的原生 Rust CLI 和无界面宿主；如何取得匹配的二进制文件或自行构建，请参阅开发环境设置。

## 使用单一宿主 {#use-one-host}

1. 在已安装 CLI 的环境中，用 `tiktools host --ipc` 启动一个无界面宿主。
2. 各终端保持相同的应用主目录。
3. 在另一终端使用 `tiktools system info`、`tiktools live status` 或 `tiktools api discover` 查看该宿主。
4. 使用文档中的关闭命令或 Ctrl-C 正常停止宿主。运行期间不要删除所有权锁。

普通 CLI 命令会连接现有宿主。`--standalone` 明确为单次调用创建一个独立核心；它不是控制已运行宿主的默认方式。

## 本地浏览器预览 {#local-browser-preview}

可选的 `local-web` 适配器能够在完成本地配对后，向浏览器提供 Vue 界面。它是开发预览，具有独立的构建与生命周期要求。它不会让 Android 成为受支持的桌面 WebView 目标，也不会授予远程用户宿主访问权限。

## 预期结果 {#expected-result}

CLI 与桌面共用控制概念、SQLite 持久化及原生事件处理。依赖平台的插件、音频、快捷键、原生窗口和凭据存储可能不可用。原生远程配置持久化目前对 Android 有明确限制。

## 恢复方法 {#recovery}

请使用 Termux 的应用私有目录，不要使用可能阻止执行文件和 Unix 套接字的 Android 共享存储。排查 IPC 故障前，先确认应用主目录一致且宿主仍在运行。如果 Android 停止了进程，请使用相同配置重新启动，并检查恢复诊断信息。文档演示无法复现 Android 的生命周期行为。

实际工具链、功能标志和设备端验证步骤见[开发者指南](../developer/index.md)。桌面程序安装仍请参阅 [Windows/Linux 指南](installation.md)。
