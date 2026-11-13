---
title: "Cloud 契约、同步与审查"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "06ae46545d1366d4f456c78a0e6583ea0338afbd496467e4d25b3abe1a5430fb"
translationRevision: "7d9e03829fdccd04da298487c8c6d81c63fd84d7e23d622a99193797d4e61dbc"
order: 180
---

# Cloud 契约、同步与审查 {#cloud-contracts-synchronization-and-review}

## 目标与前置条件 {#goal-and-prerequisites}

面向已配置的 Cloud 实例开发账户配置和目录功能。仅本地运行仍是有效模式。需要身份验证时，使用专用测试账户/后端和原生会话交接；本文档不能证明生产 bearer 或双设备流程已通过验收。

已核实的契约所有者是 [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend)，包含 [v2 配置契约](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md)及增量 [v3 接口](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md)。后端负责的可移植类型和客户端传输，在本地对应 `crates/cloud-contracts` 和 `crates/cloud-client`。桌面持久性与身份验证仍由宿主负责。

## 门控与暂存操作 {#gate-and-stage-operations}

1. 通过 `rpc.discover` 发现原生方法；较旧宿主可以保留旧视图。
2. 使用受门控的功能端点前，让宿主检查实例的 `/api/v2/capabilities`。不要将服务器声明的能力硬编码为仅存在于浏览器的标志。
3. 使用操作员配置的来源。发布构建不接受任意运行时来源替换。
4. 预览迁移和冲突，保留冻结的基准与结果不确定的写入，仅提交已审查操作。

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

这些命令仅检查 schema，不会迁移账户或保存变量值。

## 当前实现 {#current-implementation}

V2 已有原生状态/存储/协调器、受保护的 v1 迁移、冲突审查、重建及已安装插件协调路径。采纳操作明确且限定于账户；未标记的旧安装继续使用旧路径。完整双桌面验收和部分产品适配器仍未完成。账户范围的写入纪元设置会产生不可逆的服务器影响，迁移审查中对此有说明。

V3 在能力门控后增加目录、团队、集成、资源、叠加层和包接口。目录元数据不能替代签名插件验证。模板/配置导入使用所有者规则库日志；应用或导入配置时应审核 enabled。包应用会记录中断步骤，不会自动重放。Cloud 中继读取访问不能与直接协作修改权限混淆。

## 预期结果与恢复 {#expected-result-and-recovery}

配置变更保留身份、修订和审查边界。自动化更改仅在本地工作；经过审查的变量使用真实类型化原生方法，不同步原始值。叠加层更改尚未合并。冲突时比较基准/本地/最新 Cloud 记录；确认未知时，解决保留的意图。可重试 HTTP 错误绝不授权重复修改。[协作](collaboration.md)另行说明运行中宿主的权限。

## 运行细节 {#operational-details}

debug 中 `TIKTOOLS_CLOUD_ORIGIN` 优先级为运行时 → 编译值 → 默认值；release 为编译值 → 默认值，忽略运行时覆盖。空值、`off`、`none` 仅在适用配置层禁用服务。值必须为无凭据、路径、查询或片段的 HTTPS origin；HTTP 只允许显式 loopback。隔离 debug 使用 `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:8787 bun run start`；编译 release 使用 `TIKTOOLS_CLOUD_ORIGIN=https://cloud.example bun scripts/task.ts rust build -p tiktools-desktop --release --locked`。保存的另一 origin 身份不能恢复，应先检查账户与 origin。编译默认值不证明服务部署可用。

遵循[迁移及按模式恢复](../user/cloud/changes.md#operational-details)。启用不可逆 epoch 策略前协调所有设备，保留预览数量与原结果，屏障结果不明时重新读取状态，不要重复屏障来试探第一次是否执行。冲突恢复冻结审核条目与修订；审核过期需重做后再恢复，恢复本身不发送 Cloud 写入。
