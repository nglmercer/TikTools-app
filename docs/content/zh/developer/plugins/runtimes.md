---
title: "插件运行时与信任限制"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
translationRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
order: 260
---

# 插件运行时与信任限制 {#plugin-runtimes-and-trust-limits}

## 目标与前置条件 {#goal-and-prerequisites}

选择依赖和信任与宿主匹配的可执行格式。先阅读[插件概览](index.md)，针对实际构建验证清单，加载代码前检查目标特定限制。协议兼容、原生 ABI 兼容及权限授予是独立检查。

## 选择已实现的运行时 {#choose-an-implemented-runtime}

| 运行时 | 执行方式 | 信任与可移植性 |
| --- | --- | --- |
| `native` | `libloading` C ABI | 受信任的进程内代码；目标特定 |
| `process` | 独立可执行文件，带帧的 stdin/stdout | 崩溃隔离，使用普通用户操作系统权限 |
| `napi-vm` | 通过 `rquickjs` 在 QuickJS 上运行预编译 JS | 默认拒绝宿主效果；可选原生扩展会改变信任 |
| `declarative` | 经验证的集成描述符 | 没有客体程序；效果通过宿主代理 |
| `wasm` | 可选运行时槽位 | 常规构建未启用通用引擎 |

原生 ABI v1 通过小型 C 表交换序列化字节；Rust 容器、trait 对象和 future 不跨越此边界。原生库保持加载直到关闭。进程帧使用小端 u32 长度，随后是 JSON 字节，并非换行分隔的控制 RPC。诊断输出必须避开带帧的 stdout 通道。

## 理解 QuickJS 兼容性 {#understand-quickjs-compatibility}

清单名称 `napi-vm` 保持稳定，但旧文档关于独立纯 Rust 解释器的描述已属历史。客体获得文档说明的兼容全局对象，不是完整 Node 环境。不支持 `napi_get_uv_event_loop`。原生 `.node` 扩展使用有意受信任、允许列表控制及 SHA-256 验证的进程内路径；JavaScript 运行时名称不会使其变得安全。

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

完整原生扩展夹具需要不可用的平台工具时，使用针对性加载器测试。

## 预期结果与状态 {#expected-result-and-status}

清单解析会在加载前拒绝不支持的协议/ABI 组合。Android 允许可移植 QuickJS 及有效 Android/Bionic 进程入口，但拒绝原生动态库和 `.node` 扩展。桌面 Linux/glibc 产物不是 Android 进程插件。

## 恢复方法 {#recovery}

替换原生库后，重启私有宿主。如果启动超时并变为不确定状态，管理器可保留工作线程身份并阻止替换；不要宣称已成功关闭来绕过该状态。事件缓冲有界，投递可能报告滞后；启动时发出的事件不是持久队列。[加载器实现](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src)负责当前引擎及生命周期行为。
