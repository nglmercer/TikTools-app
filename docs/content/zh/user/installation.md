---
title: "在 Windows 或 Linux 上安装"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
translationRevision: "1781c02e46ca89a1a0d6452031081771da2087671f4e39db221a40b9ba5ae59b"
order: 20
---

# 在 Windows 或 Linux 上安装 {#install-windows-or-linux}

## 目标与前置条件 {#goal-and-prerequisites}

无需配置编译器即可安装已发布的桌面包。从项目的 [GitHub 发布页](https://github.com/nglmercer/TikTools-app/releases)下载，再按照你的操作系统完成安装。

<DocRequirements title="开始前的准备">

- 检查[系统要求](requirements.md)，选择与电脑兼容的安装包。
- 替换应用前，关闭所有正在运行的 TikTools 实例。
- 阅读发行说明，并保留该发行版的 `SHA256SUMS.txt`，用于验证下载文件。
- 尝试其他版本前，停止应用并备份数据目录。

</DocRequirements>

## 选择安装包 {#choose-a-package}

<DocTabs label="选择你的系统">

<template #windows>

选择 x86_64 **`installer.exe`**，通过 NSIS 为当前用户安装。桌面应用需要 **WebView2**；安装器提示时请安装。

**ZIP** 包用于便携式目录。请解压整个归档，并将可执行文件、`web/` 及所有其他打包资源保留在一起。

[查看 Windows 安装步骤](#windows)

</template>

<template #linux>

选择 **`installer.AppImage`**，在兼容 Linux 桌面上运行便携式可执行映像；选择 **`installer.deb`**，通过 Debian/Ubuntu 包管理器安装；或选择 **TAR.GZ**，使用包含可执行文件及其资源的便携式目录。

AppImage 需要执行权限。DEB 通过包管理器解析运行时依赖。请完整解压 TAR.GZ 目录，并将所有打包资源保留在一起。

[查看 Linux 安装步骤](#linux)

</template>

</DocTabs>

请确认发行版属于稳定版还是预发行版。预发行版是用于调试验证的候选版本，不代表已承诺兼容生产环境。替换现有安装前，使用该发行版的 `SHA256SUMS.txt` 检查下载内容。

## Windows {#windows}

<DocSteps>

1. 下载 x86_64 安装器并验证校验和。
2. 使用今后运行 TikTools 的用户账户启动安装器。若提示需要 WebView2，请安装。
3. 启动 TikTools。打开 **设置 → 常规** 选择界面语言。
4. 使用便携式 ZIP 时，解压整个目录。将 `web/` 及其他打包资源保留在可执行文件旁，不要仅复制并运行 EXE。

</DocSteps>

## Linux {#linux}

<DocSteps>

1. 下载匹配的 AppImage、DEB 或便携式归档，并验证校验和。
2. 使用 AppImage 时，在文件管理器中允许执行，或对下载的文件运行 `chmod +x`，然后打开。
3. 使用 DEB 时，通过系统包管理器安装，以便解析运行时依赖。
4. 使用 TAR.GZ 时，解压完整目录，保留打包资源，再启动其中的桌面可执行文件。

</DocSteps>

## 预期结果 {#expected-result}

TikTools 以离线状态启动。数据与安装文件分开保存，Windows 上通常位于 `%LOCALAPPDATA%/TikTools`，Linux 上通常位于 `~/.local/share/TikTools`。

## 恢复方法 {#recovery}

如果安装包在窗口出现前就失败，请先检查诊断信息及缺失库错误，再修改账户或迁移设置。尝试其他版本前，停止应用并备份数据目录。重新安装旧版可执行文件不会撤销数据库迁移。接下来可阅读[首次直播连接](first-live.md)或[更新指南](updates.md)。

## 运行细节 {#operational-details}

窗口始终无法打开时，查找原生日志 `tiktools.log`（控制器模式为 `controller.log`）：Windows 为 `%LOCALAPPDATA%/TikTools/logs`，Linux 为 `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`，但 `TIKTOOLS_HOME` 或 `TIKTOOLS_LOG_DIR` 可改变位置。日志初始化可能在文件创建前失败。保留可获得的终端错误，并在分享前阅读[诊断保留与隐私](preferences/troubleshooting.md#operational-details)。
