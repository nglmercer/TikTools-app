---
title: "下载与更新"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "59d259c554c7b220bb7e978e2da3165034eeb494aa28e5c73666be9945fb6869"
translationRevision: "59d259c554c7b220bb7e978e2da3165034eeb494aa28e5c73666be9945fb6869"
order: 40
---

# 下载与更新 {#downloads-and-updates}

## 目标与前置条件 {#goal-and-prerequisites}

选择正确的发布通道，并在更新时区分“检查更新”与“自动安装”。更换版本前，请关闭活动会话并备份本地数据，尤其是在跨越文档明确说明的数据库迁移时。

## 选择通道 {#select-a-channel}

[发行版列表](https://github.com/nglmercer/TikTools-app/releases)会区分稳定版与预发行版。带有 `-rc` 等后缀的候选版用于验证。应用更新器区分 stable 与 beta 源，提供版本前会验证已签名的元数据。未经签名的 JSON 文件或被修改的安装器不会被接受为有效更新。

## 检查与安装 {#check-and-install}

1. 打开 **设置 → 常规**，查看更新器当前状态与已安装版本。
2. 执行可用的检查。构建若未配置公钥或更新源能力，会显示无法检查。
3. 阅读发行说明，选择对应平台的安装包。
4. 构建若提供下载或手动交接流程，请按该流程操作。生产发布流程启用了经过身份验证的检查，但并未启用所有实验性的自动安装功能。
5. 手动更新时，关闭 TikTools，替换或安装完整且已验证的安装包，然后重新打开。数据目录应与应用文件分开保留。

## 预期结果 {#expected-result}

重新启动的应用显示所选版本，并能读取预期的本地配置。便携式 ZIP/TAR 包需要手动替换。即使存在 AppImage 安装器，未知或解压后的目录布局也可能要求手动更新。

## 恢复方法与限制 {#recovery-and-limitations}

签名检查失败即表示更新失败；请重新下载官方副本并核对发布校验和，不要绕过验证。自动替换 AppImage 和 NSIS 交接各有独立的构建门控，仍属实验性功能。系统没有通用的自动回滚机制。改用旧版二进制文件不会撤销已迁移的 SQLite 结构。

报告问题时，请提供已安装版本、安装包格式以及诊断中的更新器错误。不要附带签名密钥、邀请令牌或账户数据。[安装指南](installation.md)说明了支持的包目录布局。
