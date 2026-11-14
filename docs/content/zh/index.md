---
title: "TikTools 文档"
docId: site.home
translationKey: site.home
locale: "zh"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "25e3c15f45da9aee92bdfb6974e818f6340bb6f94a5eeda577349774cb0665a0"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / 直播工具箱

</template>

<template #intro>

# 你的直播，<br />由你做主。 {#tiktools-documentation}

连接 TikTok LIVE，将观众事件转为自动化操作，为 OBS 设计叠加层。在下方用虚构数据体验真实界面，再打造属于你的直播配置。

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">试用 TikTools</a>
[下载 TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## 下一场直播，用上 TikTools {#start-here}

准备好开播了吗？安装适用于 Windows 或 Linux 的 TikTools，连接主播，查看第一批观众事件。

1. [**查看系统要求** <span>找到适合你的系统环境。</span>](./user/requirements.md)
2. [**安装 TikTools** <span>下载并打开已发布的程序。</span>](./user/installation.md)
3. [**连接直播** <span>在事件流中查看第一批事件。</span>](./user/first-live.md)

</template>

<template #directory>

## 打造你的专属配置 {#explore-the-guides}

完成直播配置，或编写自己的插件来扩展功能。

</template>

<template #user>

### [用户指南](./user/index.md) {#user-guides}

连接直播、奖励观众，将重要时刻转为自动化操作。跟随指南，从第一次点击走到可检查的结果。

- [创建第一条自动化](./user/automations/first-rule.md)
- [设置 OBS 与叠加层](./user/overlays/index.md)
- [了解观众积分](./user/points/rules.md)

</template>

<template #developer>

### [开发者指南](./developer/index.md) {#developer-guides}

开发 Rust 宿主、Vue 前端及类型化控制 API。构建或扩展应用的开发者请从这里开始。

- [设置开发环境](./developer/setup.md)
- [探索架构与契约](./developer/architecture.md)
- [构建第一个插件](./developer/plugins/quickstart.md)

</template>

<template #resources>

## 继续探索所需的资源 {#resources}

- [**UI 组件参考** <span>17 个组件族，附交互示例、状态与源码。</span>](./developer/components/index.md)
- [**故障排查** <span>查找诊断信息，恢复常见故障。</span>](./user/preferences/troubleshooting.md)
- [**发行说明** <span>更新前先了解变化。</span>](./releases/index.md)

</template>

<template #footer>

浏览器演示运行真实的 TikTools 界面，使用虚构数据。无需安装或注册账户即可试用。指南会说明真实配置所需的条件。

[GitHub 源码](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
