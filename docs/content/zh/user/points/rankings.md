---
title: "了解排行榜与竞赛"
docId: "user.points.rankings"
translationKey: "user.points.rankings"
locale: "zh"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
translationRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
order: 30
---

# 了解排行榜与竞赛 {#understand-rankings-and-competitions}

## 目标与前置条件 {#goal-and-prerequisites}

选择合适的排名视图，并了解仍在开发的排行榜工作区有哪些限制。有意义的分数需要传入活动。如果只是查看当前观众，请先使用常规[积分](rules.md)榜或[直播事件流](../live/feed.md)中的热门观众面板。

## 使用合适的视图 {#use-the-right-view}

1. 在 **积分** 中搜索观众，按积分、等级或观众身份排序。比较手动调整与奖励前，先刷新。
2. 在 **实时动态** 中检查可用的观众排名面板。它们可以独立于积分余额表，接收排名引擎快照。
3. 需要叠加层时，打开 [叠加层](../overlays/index.md)，选择排行榜类型并检查来源选项。必须先存在已配置的排名来源，才能显示其输出。

专门的 **排名与竞赛** 工作区仍属实验性功能。其前端路由为 `#/rankings`，但它被有意隐藏于主导航之外，也不会恢复为启动标签页。侧栏没有该条目，不表示路由或排名引擎已移除。

## 配置实验性排行榜 {#configure-an-experimental-ranking}

如果环境提供该工作区，请从一个主播、一个直播间、按用户分组及会话窗口开始。添加一个已声明的指标，例如 `tiktok.like` / `like.count`，权重设为 `1`。设置适中的结果上限，并创建定义。表单也支持滚动和每日窗口，可明确设置时长/事件界限或时区偏移。

引入更多来源前，先阅读生成的快照。暂停与恢复影响排名收集；重置会有意清空排名状态。竞赛选项和投票相关来源有各自的分组规则。不要假定重置积分也会重置竞赛，或所有自定义分组都适用于所有指标。

## 预期结果与状态 {#expected-result-and-status}

常规排行榜可用，专用管理页面仍属实验性。分数来自宿主，不是通过统计显示的事件行得出。本指南不能证明真实直播/OBS 或每种竞赛组合都已通过验收。

## 恢复方法 {#recovery}

分数持续为空时，检查主播、直播间、来源启用状态、窗口及暂停状态。移除不受支持的来源/分组组合，不要编造字段名。重置前通过[备份](../preferences/backups.md)保存重要数据；报告问题时附上定义 ID 与诊断代码。
