---
title: "按日期查看统计数据"
docId: "user.points.statistics"
translationKey: "user.points.statistics"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a7333d75baa9aec6e0c4afb69d1c773575a56ee8da982f0fa5a11a73b903ec2d"
translationRevision: "65a78c17935897b77577b4ac2f50610122de509350e14d8f1e7addced9a13e84"
order: 20
---

# 按日期查看统计数据 {#review-statistics-across-dates}

## 目标与前置条件 {#goal-and-prerequisites}

在有用的日期范围内比较某位主播的活动，并理解统计总量为何与可见事件流或积分余额不同。你需要本地安装中已有记录，并通过 [连接](../live/connections.md) 选中目标主播。Cloud 登录不是这些本地分析的数据来源。

## 查询已记录的时段 {#query-the-recorded-period}

1. 打开 **数据分析**，确认日期控件旁的主播。所选主播决定查询哪些已保存的聚合数据。
2. 选择 **今天**、`7d`、`30d`、`90d` 或自定义范围。使用自定义日期时，明确设置起止日期并检查先后顺序。
3. 比较结果前，先阅读显示的日期跨度及会话数量。本地日期边界使用时区偏移；更改电脑时区可能改变时段的解释方式。
4. 选择需要的指标：聊天、礼物、点赞、钻石或观众峰值。使用活动图表比较时段，在贡献者表格中查看返回的领先观众。
5. 在贡献者表格中搜索已经返回的观众。这不是对所有历史观众进行无限制搜索。
6. 新活动发生后或返回页面时，选择 **刷新**。分享任何数字时，一并记录主播、日期及指标。

当前页面以主播和日期跨度调用原生 `analytics.summary` 操作，读取包含以往记录会话的持久化聚合数据，不局限于当前事件流保留的事件。统计数据库保存聚合数据，不保存聊天逐字记录。

## 预期结果与状态 {#expected-result-and-status}

本地持久化分析及日期范围查询均可使用。没有活动的日期返回空结果是正常情况。积分余额可能不同，因为[获取倍率](rules.md)、手动调整与自动化奖励衡量的是其他内容。钻石、礼物数量和积分不能互换单位。

## 恢复方法 {#recovery}

数据看似缺失时，请清空贡献者搜索、扩大日期范围、确认主播并刷新。宿主错误后保留诊断代码，待连接恢复再重试。清空事件流不会擦除统计。新的数据目录没有先前历史；迁移安装时请使用[备份](../preferences/backups.md)。

<DemoFrame scenario="analytics" locale="zh" title="演示：统计分析" />

## 运行细节 {#operational-details}

Analytics 当前请求所选主播及日期范围内的前 10 位贡献者。搜索仅筛选收到的子集，不是全局观众查询。这些持久聚合的时间范围与单位不同于动态流会话内的钻石/点赞榜单，也不同于积分余额。导出或比较结果时，请同时保留主播、日期和指标。
