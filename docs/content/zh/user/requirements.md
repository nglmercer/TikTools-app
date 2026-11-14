---
title: "系统要求"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
translationRevision: "4119dfbcd58aeb11c7db0d37bf2a076896d60eb5d45ea7179b0ad3768a2c67be"
order: 10
---

# 系统要求 {#requirements}

## 目标 {#goal}

安装前，选择受支持的桌面安装包，并确认预期工作流程需要哪些服务。

## 所需条件 {#what-you-need}

- 使用已发布的桌面包需要 Windows x86_64 或 Linux x86_64 电脑。当前发布流程构建这两个目标，不发布 macOS 桌面版或 Android APK。
- 可正常使用的图形会话。Windows 需要 Microsoft WebView2 Runtime。Linux 需要兼容的 GTK/WebKitGTK 环境；DEB 声明了运行时依赖包，AppImage 则包含经过审查的媒体辅助程序。
- 实际连接直播需要能够访问 TikTok 的网络。直播间未开播、访客会话过期或上游限流，无法通过修改自动化规则来解决。
- 在安装目录以外提供空间，保存 SQLite 数据、插件和日志。应用会确定每个用户的数据目录；移动可执行文件不会同时移动这些数据。

基于账户的目录或协作操作需要 Cloud 账户，但并非每个本地编辑器都需要账户。只有在直播中显示小组件时才需要 OBS。音频/TTS 集成可能需要单独安装插件，并配置其提供商或服务。

## 检查环境 {#check-the-environment}

1. 选择与你的操作系统及 x86_64 架构匹配的安装包。
2. Windows 安装器提示缺少 WebView2 时，请安装它。Linux 缺少运行库时，请通过发行版的包管理器解决。
3. 先确认应用能够打开窗口，再连接 TikTok 或添加插件。
4. 继续阅读[安装指南](installation.md)，然后完成[首次直播连接](first-live.md)。

## 预期结果 {#expected-result}

应用能够启动，离线时也能使用主导航。可选集成可能需要相应插件或宿主能力才能使用。

## 恢复方法 {#recovery}

如果窗口空白或程序提前退出，请收集启动诊断信息和确切的安装包版本。AppImage 与 DEB 是不同的安装格式，都不是 Android 可执行文件。该环境请参阅 [Termux 与无界面运行的适用范围](termux.md)。从源码构建的额外要求见开发者指南。

## 运行细节 {#operational-details}

窗口始终无法打开时，查找原生日志 `tiktools.log`（控制器模式为 `controller.log`）：Windows 为 `%LOCALAPPDATA%/TikTools/logs`，Linux 为 `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`，但 `TIKTOOLS_HOME` 或 `TIKTOOLS_LOG_DIR` 可改变位置。日志初始化可能在文件创建前失败。保留可获得的终端错误，并在分享前阅读[诊断保留与隐私](preferences/troubleshooting.md#operational-details)。
