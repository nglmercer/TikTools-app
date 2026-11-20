---
title: "当前传输方式与平台限制"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "e061718f08cd5971f353da7709d4ea21feae8b6e2962b55de82e86544ca28209"
order: 200
---

# 当前传输方式与平台限制 {#current-transports-and-platform-restrictions}

## 目标与前置条件 {#goal-and-prerequisites}

选择目标构建和平台上实际存在的传输方式。先阅读 [Control API](control-api.md) 和[协作](collaboration.md)。传输负责承载序列化操作，不能引入第二个核心，也不能自行授予权限。

## 选择边界 {#choose-the-boundary}

| 路径 | 当前实现 | 重要限制 |
| --- | --- | --- |
| 桌面 WebView | Wry IPC 与共享控制客户端 | GUI/线程生命周期归桌面端所有 |
| 本地 CLI | Unix socket 或 Windows 命名管道；显式 stdio 主机 | 同一应用目录与所有者 |
| 本地浏览器 | 原生回环适配器加 `browser-transport.ts` | 已配对的原生主机，不是公开远程服务器 |
| 已认证的 HTTPS/WSS 网关 | 带 TLS、hello、授权与特性门控的 SDK/浏览器传输 | 远程命令需要显式授权 |
| 可安装的远程控制插件 | 基于 WSS 中继、使用 `remote.controller.ticket` 的 `tiktools.remote-control` | 由插件拥有；主机不编译任何远程代码 |

TypeScript 传输接口位于 `src/web/shared/api/control-transport.ts`。远程控制是可安装插件，不属于主机二进制文件：每条命令都要经过 `HostCallPolicy`（`remote.command.dispatch`、`remote.pair.list`、`remote.grant.list`、`remote.grant.revoke`、`remote.controller.ticket`）。此构建不包含原生 peer/RTC 传输，也没有 Cloud Teams 主机链接。

## 检查所选路由 {#inspect-the-selected-route}

在配对或显示控制操作前，先查询已连接主机支持的方法与能力：

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

配对与授权由插件负责：控制器票据短期有效且只能使用一次，`remote.grant.list` / `remote.grant.revoke` 是权威视图。中继连接只证明传输可用，绝不代表拥有 RPC 权限。

## 预期结果与状态 {#expected-result-and-status}

本地 IPC 与桌面传输已是稳定路径。远程控制通过可安装插件和已认证中继工作，不是第二个主机运行时。登录码会以五分钟有效期中继给已连接设备；未配置账户时回退到 OAuth。Android 本地 IPC 可配合插桩版 `mobile` 工具（`info` / `exec` / `tty`）使用；桌面原生插件与 WebView 在该平台仍不可用。

## 恢复 {#recovery}

对于已接受的操作，断连可能意味着结果不确定。重连时必须作废旧投影、重新协商身份，并恢复或快照有界流。切勿降级证书校验，也不要在远程失败后静默回退到本地所有者。过期的控制器票据和已撤销的授权必须 fail closed。

## 运维细节 {#operational-details}

授权应遵循最小权限并设时限，每次重连后都要重新检查。分别排查身份、票据过期、授权范围、中继可达性与帧大小。超大帧会在帧校验处失败，不会变成部分 RPC。在真实双设备验收前，先用 fixture 测试变更或撤销的会话。
