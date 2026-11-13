---
title: "在 Windows 或 Linux 上安装"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
translationRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
order: 20
---

# 在 Windows 或 Linux 上安装 {#install-windows-or-linux}

## 目标与前置条件 {#goal-and-prerequisites}

无需配置编译器即可安装已发布的桌面包。请先检查[系统要求](requirements.md)，关闭正在运行的 TikTools，并从项目的 [GitHub 发布页](https://github.com/nglmercer/TikTools-app/releases)下载。

## 选择安装包 {#choose-a-package}

| 安装包 | 用途 |
| --- | --- |
| Windows `installer.exe` | 面向当前用户的 NSIS 安装；需要 WebView2。 |
| Linux `installer.AppImage` | 用于兼容 Linux 桌面的便携式可执行映像。 |
| Linux `installer.deb` | 通过 Debian/Ubuntu 包管理器安装。 |
| Windows ZIP / Linux TAR.GZ | 包含可执行文件及其资源的便携式目录。 |

请确认发行版属于稳定版还是预发行版。预发行版是用于调试验证的候选版本，不代表已承诺兼容生产环境。替换现有安装前，使用该发行版的 `SHA256SUMS.txt` 检查下载内容。

## Windows {#windows}

1. 下载 x86_64 安装器并验证校验和。
2. 使用今后运行 TikTools 的用户账户启动安装器。若提示需要 WebView2，请安装。
3. 启动 TikTools。打开 **设置 → 常规** 选择界面语言。
4. 使用便携式 ZIP 时，解压整个目录。将 `web/` 及其他打包资源保留在可执行文件旁，不要仅复制并运行 EXE。

## Linux {#linux}

1. 下载匹配的 AppImage、DEB 或便携式归档，并验证校验和。
2. 使用 AppImage 时，在文件管理器中允许执行，或对下载的文件运行 `chmod +x`，然后打开。
3. 使用 DEB 时，通过系统包管理器安装，以便解析运行时依赖。
4. 使用 TAR.GZ 时，解压完整目录，保留打包资源，再启动其中的桌面可执行文件。

## 预期结果 {#expected-result}

TikTools 以离线状态启动。数据与安装文件分开保存，Windows 上通常位于 `%LOCALAPPDATA%/TikTools`，Linux 上通常位于 `~/.local/share/TikTools`。

## 恢复方法 {#recovery}

如果安装包在窗口出现前就失败，请先检查诊断信息及缺失库错误，再修改账户或迁移设置。尝试其他版本前，停止应用并备份数据目录。重新安装旧版可执行文件不会撤销数据库迁移。接下来可阅读[首次直播连接](first-live.md)或[更新指南](updates.md)。
