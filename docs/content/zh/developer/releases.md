---
title: "发布、安装包与更新器门控"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
translationRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
order: 220
---

# 发布、安装包与更新器门控 {#releases-packages-and-updater-gates}

## 目标与前置条件 {#goal-and-prerequisites}

准备可审查的发布产物，并区分已验证下载与自动安装。这是维护者流程，需要相应平台工具链和受保护的签名材料。普通贡献者可以检查打包与版本校验，无需发布或创建发布标签。

## 准备一致的产物 {#prepare-consistent-artifacts}

`package.json`、工作区 `Cargo.toml` 和 `Cargo.lock` 共用应用版本。使用任务运行器，不要单独编辑某一个版本：

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

维护者有意更改版本时，使用 `bun scripts/task.ts version bump v4.2.3`，并替换为选定的下一版本。完整构建还会创建小组件资源。发布打包将原生可执行文件放在 `web/index.html` 旁，并发布校验和。安装器流程还会构建 NSIS、DEB 和 AppImage，并对照便携式暂存验证资源。插件包仍作为独立运行时安装。

## 审查更新器信任 {#review-updater-trust}

`tiktools-app-update` 同时使用签名通道元数据及签名包字节。可信公钥、更新源启用和已发布且验证的资源是独立要求。仅有包 SHA256 校验和不会启用应用更新器。应用更新密钥与插件发布者密钥应分开。

宿主限制下载和重定向，安装交接必须等待核心/WebView 安全关闭。AppImage 安装还需构建时设置 `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1`。NSIS 有独立的 `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1` 门控。已有代码及签名单元测试，并不足以支持在发布构建中启用任一门控。

## 预期结果与状态 {#expected-result-and-status}

便携式打包和平台安装器工具已实现。检查/下载依赖配置；自动替换和重启仍属实验性。未知或解压后的 Linux 布局回退到手动安装。启动 NSIS 安装器，不代表正在退出的旧进程可以报告安装成功。

## 验证与恢复 {#verify-and-recover}

运行 `bun scripts/task.ts rust test -p tiktools-app-update --locked` 验证本地协议/签名夹具。真实升级、失败、配置保留及重启验收，需要可丢弃的受支持安装。保留前一个已验证产物；更新源/附属文件发布中断时必须保持封闭失败。插件关闭仍不确定时，不要重新启用交接。[发布工作流程](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml)和更新器源码负责当前门控；历史发布计划不是部署检查表。
