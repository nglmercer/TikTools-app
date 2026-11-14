---
title: "当前传输方式与平台限制"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "zh"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6696987a0db16b3b4d364c6136d0e552dbebd6fc68346e3ed031587a7b004468"
translationRevision: "170fec34f0552f33947038429c504c2ca8720074fc76079eb61b3848439f4897"
order: 200
---

# 当前传输方式与平台限制 {#current-transports-and-platform-restrictions}

## 目标与前置条件 {#goal-and-prerequisites}

选择目标构建和平台上实际存在的传输方式。先阅读 [Control API](control-api.md) 和[协作](collaboration.md)。传输负责承载序列化操作，不能引入第二个核心，也不能自行授予权限。

## 选择边界 {#choose-the-boundary}

| 路径 | 当前实现 | 重要限制 |
| --- | --- | --- |
| 桌面 WebView | Wry IPC 与共享控制客户端 | GUI/线程生命周期属于桌面 |
| 本地 CLI | Unix 套接字或 Windows 命名管道；明确的 stdio 宿主 | 使用同一应用主目录及所有者 |
| 本地浏览器 | 原生回环适配器及 `browser-transport.ts` | 配对的原生宿主，并非模拟对象或公开远程服务器 |
| 直接远程 | 经身份验证的 HTTPS/WSS 网关及 SDK/浏览器传输 | TLS、hello、授权与功能门控 |
| 原生对等连接 | `tiktools-peer`，加密 iroh/QUIC 流 | 对等身份不等于 RPC 权限 |
| 原生 WebRTC | `tiktools-rtc`，签名信令与数据通道 | 仍须进行协商、授权与路由检查 |

TypeScript 传输接口位于 `src/web/shared/api/control-transport.ts`。原生对等配置不会隐式启用公开第三方中继或发现。WebRTC 是原生 Rust 实现（`webrtc` 0.21.1），不依赖桌面 UI 中的浏览器 `RTCPeerConnection`。

## 检查选定路由 {#inspect-the-selected-route}

配对或显示控制端操作前，先发现所连宿主支持的方法与能力：

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` 观察选定的 ICE 候选对，以区分直连与中继。仅有就绪状态不能证明是直连。Cloud 信令/中继连接机制与业务授权相互独立，必须保留原生授权检查。

## 预期结果与状态 {#expected-result-and-status}

本地 IPC 和桌面传输是成熟路径。原生对等、WebRTC 和控制端集成已有源码及针对性测试，但部署与多电脑验收仍属实验性。旧文档声称没有对等/中继实现的内容已属历史。Android 本地 IPC 可用；持久化远程配置尚未选定凭据存储，桌面原生扩展和 WebView 在该平台仍不可用。

## 恢复方法 {#recovery}

断线后，应将已接受操作视为可能存在不确定结果。重连必须使私有投影失效、重新协商身份，并恢复有界流或获取快照。不要降低证书检查要求，也不要在远程失败后静默回退到本地所有者。[RTC 实现](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src)及实际功能依赖图优先于旧传输计划。

## 运行细节 {#operational-details}

Peer 使用 ALPN `tiktools/collaboration/1`，当前连接超时为 15 秒，最多显式配置四个 relay。Peer/RTC 帧上限为 1 MiB；RTC 拆分为最多 16 KiB 的数据包。超大帧会在验证中失败，不产生部分 RPC。应分别诊断配置/身份、信令、连接超时、帧大小及授权。Cloud relay 不是 ICE TURN 服务器；空 relay 列表不启用公共发现。测试路径前检查宿主/设备活动及当前已确认授权，并记录 ICE 选择直连还是 relay。ready/加密传输不证明 RPC 权限，也不保证穿越所有 NAT。真实双机验收前，用 fixtures 测试会话变化/撤销和超大帧。
