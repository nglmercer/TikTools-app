---
title: "前端分层与数据流"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40ff5183f34bff831db48c76fef50f9ef8d5fe40b632ed77747c10ebad708fb3"
translationRevision: "4acd1a58046c3f7795b5ac2361926376a517f63f67f55786e8534dcf7b6ee065"
order: 40
---

# 前端分层与数据流 {#frontend-layers-and-data-flow}

## 目标与前置条件 {#goal-and-prerequisites}

扩展 Vue 应用，同时避免页面耦合原生 IPC。前端根据已安装工作区的版本范围使用 Vue 3.5、Pinia 4、Vue Router 5 和 vue-i18n 11。使用锁定依赖及[环境设置](setup.md)中的命令，不要新建另一套前端脚手架。

## 找到所属层 {#find-the-owning-layer}

当前 Feature-Sliced Design 布局位于 `src/web/`：

| 层 | 职责 | 现有示例 |
| --- | --- | --- |
| `app` | 引导、路由、外壳及运行时生命周期 | `app/model/use-app-runtime.ts` |
| `pages` | 路由组合 | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | 较大型组合面板 | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | 用户操作及其草稿 | `features/apply-rule-template/` |
| `entities` | 领域 store 与可复用领域逻辑 | `entities/rule-template/` |
| `shared` | API、基础 UI、i18n 及通用工具 | `shared/api/control-client.ts` |

高层由低层组合而成。宿主调用应通过共享控制客户端；传输选择属于引导流程。旧文章中的 `src/web/platform` 和 `src/web/views` 已是历史位置。

## 实现范围明确的改动 {#implement-a-focused-change}

1. 在 `app/router/routes.ts` 找到路由，再沿页面定位 feature 和 entity store。
2. 字段、状态或对话框使用现有共享组件，保留标签、焦点行为和事件契约。
3. 将可编辑候选项与已保存的宿主记录分开。使用 `shared/lib/request-scope.ts` 忽略过期异步响应；使请求失效并不会取消宿主已经接受的修改。
4. 通过 `shared/i18n` 添加语言键，并在所属生命周期结束时释放订阅。

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

路由渲染当前宿主数据，冲突时保留未保存编辑。排名与竞赛已有源码和路由，但仍隐藏于常规导航；不能仅凭组件存在推断产品功能已开放。原生、本地浏览器和演示引导路径可以提供不同传输，同时复用相同功能代码。

过期响应覆盖较新查询时，先检查其代次，再修改后端。路由失去插件页面时，按运行时现有行为返回已安装插件视图。[应用运行时](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts)是附加、重新同步和释放资源的参考。

## 运行细节 {#operational-details}

bootstrap 选择真实/demo ControlClient，`mountWorkspace({client,locale,target})` 安装 Pinia、i18n、router 及 `controlClientKey`；AppShell 通过 `useAppRuntime` 启动订阅、gap 重同步与呈现。即使存在 Pinia/i18n，缺少 provider 的 `useControlClient` 仍会抛错。应用复用挂载工厂，组件测试明确提供客户端；结束时卸载并 detach。entities 内不选择传输。

await 前创建请求 ticket 与活动退出函数，只有 ticket 仍有效才应用响应，并始终在 finally 退出活动。上下文变化或卸载时 invalidate。重叠操作各自持有退出函数，避免一个响应清除另一项 busy。ticket 失效并不取消远程工作。

```ts
import { createRequestScope, createActivityCounter } from '@/shared/lib/request-scope.ts';
const requests = createRequestScope();
const activity = createActivityCounter();
async function refresh() {
  const ticket = requests.begin();
  const leave = activity.enter();
  try {
    const result = await client.call('system.info', {});
    if (ticket.isCurrent()) info.value = result;
  } finally { leave(); }
}
// onBeforeUnmount(() => { requests.invalidate(); activity.reset(); });
```

```ts
import { mountWorkspace } from '@/app/mount-workspace.ts';
const app = await mountWorkspace({ client, locale: 'en', target: '#app' });
// At teardown:
app.unmount();
client.detach();
```
