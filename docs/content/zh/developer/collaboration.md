---
title: "协作权限与身份验证"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb71673a581e923e3c02ae401a46937b007f9919690ca020d22c27b370b6c61d"
translationRevision: "c13d4e37e61a1a45537a43ba8304c2eeb1beae2e49d36651785d6059977939a5"
order: 190
---

# 协作权限与身份验证 {#collaboration-authority-and-authentication}

## 目标与前置条件 {#goal-and-prerequisites}

允许获批准的设备查看现有宿主，或在其上执行范围有限的操作。使用测试所有者配置、支持远程的构建及已配置传输。账户登录、Cloud 团队成员身份、本地所有权和直接委托会话是不同的权限来源。

## 建立信任链 {#establish-the-trust-chain}

本地所有者的 `collaboration.*` 管理负责配置、邀请、待处理请求、主体、授权、会话及审计。配置监听器或创建凭据前，先发现 schema：

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

邀请启动配对，本身不授予业务权限。所有者批准后才创建对应会话/授权。直接 HTTPS 需要经过验证的证书，或明确固定的原生信任。原生 bearer 与浏览器 Cookie 的边界不同。初始 `collaboration.hello` 绑定协议、宿主实例、主体、会话、授权修订、能力及协商限制。

## 逐项授权操作 {#authorize-each-operation}

支持的远程方法必须通过允许列表、能力、资源限制及当前会话检查。工作流程禁用/删除使用持久化比较并交换元数据。积分调整具有累加性质，并有独立授权限制。远程自动化读取只返回封闭式摘要；禁用且归作者所有的草稿，不会仅因保存就获得可执行的所有者权限。

所有者审查/采纳及经过检查的批准，会绑定当前定义、依赖及来源。预览清单报告的是检查数据，不是执行令牌。应在执行边界重新验证授权和资源绑定，不能把传输身份当成永久权限。

## 预期结果与状态 {#expected-result-and-status}

直接原生协作已实现限定范围读取、部分持久化修改及有界恢复。Cloud 团队/中继另有各自适配器；文档中的 Cloud 中继读取路径不能被描述为完整管理或草稿执行。其余委托效果、领域写入和多电脑验收仍属实验性。选定路径见[远程客户端](remote-client.md)和 [Cloud](cloud.md)。

## 恢复与验证 {#recover-and-verify}

撤销或身份改变时，停止订阅并丢弃私有投影。按协商领域和游标有效性使用 `collaboration.subscribe`、`snapshot` 和 `resume`。确认不确定时，查询原始命令 ID。不要换新 ID 重试删除，也不要根据旧回执恢复权限。[授权源码](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src)和注册表策略定义边界；仅通过 TLS 不是授权测试。

## 运行细节 {#operational-details}

Teams 流程：注册设备、发出/加入邀请或访问请求、批准，再由宿主确认原生权限的当前修订。`pendingHost` 表示最后一步未完成。新修订使旧确认失效；expectedRevision 过期时重新加载。原生接收要求新快照、活动宿主/设备、有效权限且 confirmedRevision 等于 revision。UI 提供宿主/加入面板及可用批准入口；原生确认是 API 契约，不保证所有构建都有相应按钮。

旧角色映射为封闭集合：observer 获得 live/workflows/积分/竞赛/自动化/结果读取；moderator 额外获得积分调整、workflow/自动化开关及竞赛管理；automationEditor 在读取外增加自动化编辑。预设不授予任意脚本、外部副作用或可复用自动化委派。显式 `nativeGrant` 要求 `custom`，并在那里限制资源、副作用范围和积分 delta。原生客户端只支持 `desktop` 或 `agent`。不支持的策略直接失败，不会扩大权限。分别诊断配对、宿主确认、修订新鲜度和传输。
