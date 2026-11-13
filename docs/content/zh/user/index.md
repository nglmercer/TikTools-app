---
title: "使用 TikTools"
docId: "user.overview"
translationKey: "user.overview"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
translationRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
order: 0
---

# 使用 TikTools {#use-tiktools}

TikTools 可将 Windows 或 Linux 桌面连接到 TikTok LIVE，显示直播事件、记录观众积分，并在你的电脑上执行规则。桌面界面与可选的无界面宿主共用同一个 Rust 核心。Cloud 账户与 TikTok 连接相互独立。

## 选择你的第一项任务 {#choose-your-first-task}

1. 查看[系统要求](requirements.md)，然后[安装已发布的程序包](installation.md)。使用桌面发行版无需安装 Rust、Cargo 或 Bun。
2. [连接直播](first-live.md)。如果直播间允许访客访问，先尝试不提供 TikTok Cookie。
3. 使用 **实时动态** 查看事件。**积分** 记录观众总积分，**数据分析** 显示统计数据，**自动化** 将触发条件与已配置的操作关联起来。
4. 在 **插件** 中安装可选集成。外部插件包与应用程序分别分发。
5. 使用 **叠加层** 为 OBS 准备浏览器源。实时传送小组件需要网关集成。

## 可用功能与实验性功能 {#available-and-experimental-features}

主导航包含实时动态、积分、数据分析、连接、自动化、插件、叠加层和设置。排行榜与竞赛已有原生引擎和可直接访问的路由，但其管理页面仍未出现在常规导航中。旧设计文档中描述的原型不一定都已发布。

Cloud 插件变更和经审查的变量使用各自明确的分区控制项。自动化目前与本地参考副本比较；该按钮不会将配置发布到 Cloud。协作传输与部分安装器交接流程仍属实验性功能。控件被禁用通常表示宿主缺少相应能力，或尚未满足前置条件。

## 遇到故障时 {#when-something-fails}

打开 **诊断**，保留错误来源和请求标识符，并记录已安装的应用版本。不要包含 Cookie、邀请令牌或私有变量值。先按各篇指南中的恢复步骤处理，再查阅支持指南。交互式演示使用虚构数据，无法验证你的 TikTok 账户、OBS 或网络是否正常。

如需开发源代码，请参阅独立的[开发者文档](../developer/index.md)。
