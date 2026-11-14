---
title: "远程客户端与恢复"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3fa6b05b6c19548fa64d45b3114bfd8080f3f0ed510f835c4d18297d9ccd6cae"
translationRevision: "7399ab332e988f65ef680fc660db25d0fa98267b0f4a469073f7b327bdfc84a4"
order: 90
---

# 远程客户端与恢复 {#remote-clients-and-recovery}

## 目标与前置条件 {#goal-and-prerequisites}

将控制端连接到宿主，同时不创建本地直播核心。你需要启用了相关远程功能的宿主、所有者批准的邀请、可访问的 HTTPS 来源，以及受支持的安全凭据存储。授权见[协作](collaboration.md)，原生路由选择见[传输](transports.md)。

## 配对与检查 {#pair-and-inspect}

以下命令用于手动测试环境；将示例来源替换为已配置的宿主：

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

自签名宿主需通过独立渠道与所有者比较指纹，并在配对时传入 `--fingerprint`。检查不会发送经过身份验证的业务请求，也不会批准信任。邀请和凭据值通过支持的提示输入，不应作为命令行秘密传递。

配置文件包含身份、信任策略和不透明的安全存储引用。Linux 使用 Secret Service，macOS 使用 Keychain，Windows 使用 Credential Manager。存储不可用时明确失败；Android 尚未选定持久化配置后端。SDK 可使用明确提供的内存凭据。`remote forget` 删除本地凭据及元数据，但不会撤销宿主会话。

## 保留命令身份 {#preserve-command-identity}

执行受支持的修改前，先读取资源及元数据。保留其基准修订、宿主实例和持久化命令 ID。提交后连接消失时，使用原始 ID 查询 `collaboration.commands.get`。回执是历史记录；当前资源需单独刷新。工作流程禁用/删除、积分调整及禁用且归作者所有的自动化草稿，各有不同契约和授权。

## 预期结果与状态 {#expected-result-and-status}

原生桌面有隔离的 `tiktools-desktop --controller` 入口及绑定配置的日志。控制端覆盖仍属实验性。配对、传输连接或宽泛功能标志，不授权任意编辑。远程自动化读取公开的是封闭式摘要，绝不是可编辑源码；远程启用/触发与本地所有者采纳仍相互独立。

## 恢复方法 {#recover}

序列缺口出现时，使用协商的订阅快照/恢复路径；身份变化时清空私有投影。重连后不要自动重放业务修改。证书变化需要明确的信任审查。[配置实现](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs)说明配置租约和刷新不确定性；真实多电脑与操作系统凭据存储验收，必须与回环测试分开报告。

## 运行细节 {#operational-details}

先将可靠事件应用到对应快照/投影，再调用 `acknowledge`；提前确认会丢失恢复证据。重连时只可在宿主实例、订阅、会话及授权上下文相同的情况下续传。出现 gap、上下文变化、游标拒绝或浏览器因 gap 关闭时，应重新获取已授权快照并建立新订阅。不要从事件历史重放变更。不确定写入使用[元数据与原始命令流程](control-api.md#operational-details)。

本地 `event` 包含 `{topic,data?}`，无数据的变体可以省略 data。远程 `event.params` 是 ReliableEvent，含字符串序列、事件 ID、资源版本和 actor。`event.gap` 是独立方法。以下信封仅为格式示例，不表示可持久重放丢失副作用。

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

以下 SDK 集成示意需要 remote-profiles。投影回调由应用实现，必须在 acknowledge 前完成。参见 [RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) 中的类型化基线、游标和续传契约。

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```
