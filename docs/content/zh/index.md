---
title: "TikTools 文档"
docId: site.home
translationKey: site.home
locale: "zh"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "31c9a7b19885a3e31e5f71e0dfbd7757513cff9330b60145b3302225e9207bcb"
translationRevision: "31c9a7b19885a3e31e5f71e0dfbd7757513cff9330b60145b3302225e9207bcb"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / 文档

</template>

<template #intro>

# 让每一场直播<br />更出色的实用指南。 {#tiktools-documentation}

连接 TikTok LIVE，让观众事件发挥作用，并基于 TikTools 构建新功能。从首次连接到下一个插件，都有实用文档伴你前行。

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">试用交互式演示</a>
[下载 TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## 一步步开启首次直播 {#start-here}

从已发布的应用开始，无需开发工具。

1. [**查看系统要求** <span>找到适合你的系统环境。</span>](./user/requirements.md)
2. [**安装 TikTools** <span>下载并打开已发布的程序。</span>](./user/installation.md)
3. [**连接直播** <span>在事件流中查看第一批事件。</span>](./user/first-live.md)

</template>

<template #directory>

## 找到适合你的起点 {#explore-the-guides}

两条学习路径，共用一套工具。

</template>

<template #user>

### [用户指南](./user/index.md) {#user-guides}

配置直播、回应观众，让日常任务更轻松。使用应用的用户请从这里开始。

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

## 开发与使用中的实用资源 {#resources}

- [**UI 组件参考** <span>17 个组件族，附示例与源码。</span>](./developer/components/index.md)
- [**故障排查** <span>查找诊断信息，恢复常见故障。</span>](./user/preferences/troubleshooting.md)
- [**发行说明** <span>更新前先了解变化。</span>](./releases/index.md)

</template>

<template #footer>

演示使用虚构数据。每篇指南都会注明实验性功能及前置条件。

[GitHub 源码](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
