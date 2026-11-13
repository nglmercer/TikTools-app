---
title: "生命周期、所有权与清理"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
translationRevision: "92e2164c28be287f8b5d5c96d787a106bb011e1939ff490e4e69737c87bb9716"
order: 50
---

# 生命周期、所有权与清理 {#lifecycle-ownership-and-cleanup}

## 目标与前置条件 {#goal-and-prerequisites}

确保挂载、重连和关闭都会释放各自拥有的资源。先了解[架构](architecture.md)，原生生命周期实验使用隔离的 `TIKTOOLS_HOME`。前端取消订阅与关闭宿主解决的是不同所有权问题。

## 理解桌面生命周期 {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` 实现 Winit 的应用处理器。窗口和 WebView 在事件循环线程创建。窗口开始时隐藏，Vue 发送 `frontend-ready` 后才显示。存在托盘时，关闭窗口会将其隐藏；没有托盘则开始关闭流程。退出时，会先请求核心停止服务，再结束事件循环。

`crates/tiktools-core/src/shutdown.rs` 会尽早关闭插件启动准入，停止后台协调和监控，断开直播，并排空所属工作。重复的关闭调用者等待同一次完成。丢弃 future 无法停止任意原生代码，也无法撤销外部效果。

## 明确管理前端资源 {#own-frontend-resources-explicitly}

沿用 `use-app-runtime.ts` 的模式：

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // 在此刷新所属 store 中已提交的展示状态。
});
onUnmounted(stop);
```

回调内容属于拥有该视图的 entity。feature 应释放其订阅、定时器、动画帧和请求作用域，而应用引导负责附加和分离共享传输。小组件模板变化时，必须释放旧控制器，不能让其提醒定时器继续运行。

## 预期结果与验证 {#expected-result-and-verification}

离开页面再返回不会成倍增加回调。隐藏的展示停止不必要的渲染，但不暂停权威记账。重连会报告缺口并重新加载快照，不会重放业务效果。针对性示例可运行 `bun scripts/task.ts test web cloud-lifecycle` 和 `bun scripts/task.ts test web request-scope`。

## 恢复与限制 {#recovery-and-limits}

QuickJS 启动超时后可能仍处于不确定状态；管理器会保留工作线程身份并阻止替换，不会假装已经停止。部分同步插件生命周期控制路径仍可能阻塞调用者。这些属于实现限制，尤其需要在更新器交接时考虑。应正常停止拥有资源的宿主，不要删除运行中的锁。[关闭源码](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs)及运行时测试，比历史生命周期图更有证明力。
