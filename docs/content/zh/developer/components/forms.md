---
title: "表单"
docId: "developer.components.forms"
translationKey: "developer.components.forms"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
translationRevision: "8771fa42ddee98119c9c2ad1eb1060e440bab919b874ffce907c9d6716456f6f"
---

# 表单 {#forms}

## 用途与前置条件 {#purpose}

将标签、提示和错误关联到实际控件。表单行同时支持自动关联和明确传递的作用域属性。使用现有 Vue 工作区及其 i18n 提供器。预览在隔离的应用框架中挂载真实共享组件；文档主题不会加载桌面的全局 CSS。

## 试用示例 {#example}

1. 加载预览，或打开完整演示。
2. 切换状态选择器，在适用时检查加载中、空白、错误、禁用、长文本及移动端表现。若某种状态没有独特渲染，不会为组件人为添加行为。
3. 根据控件使用 Tab、Enter、Escape 和方向键。检查无障碍名称、可见焦点，以及关闭弹出内容后焦点是否返回。
4. 修改值或调用操作，检查得到的选择或输出。刷新或重置演示会恢复演示状态。

<DemoFrame scenario="settings" locale="zh" route="/components/forms" title="表单示例" />

## API {#api}

以下 props、默认值、事件、插槽和模型声明从组件源码生成。导入的类型保留与源码的关联，不会手工复制。未声明属性遵循组件的转发契约；给包装元素绑定原生事件前，请先审查源码。

<!--@include: ../../../../.generated/components/forms.zh.md-->

## 可执行源码 {#source}

下面的同一文件既编译用于预览，也作为代码片段包含在此。少量状态属于示例本身。账户、插件和宿主逻辑应留在 features/entities 中，不应放入共享 UI。

<<< ../../../../examples/forms.vue

## 无障碍与恢复 {#accessibility}

使用可见标签或 aria-label，保留键盘操作，并避免只依赖颜色传达信息。在窄屏和较长的翻译标签下测试。对于表单，将行的 controlAttrs 绑定到实际输入控件，确保错误和必填状态仍有关联。禁用控件本身无法解释故障；应在旁边显示状态及恢复操作。

预览失败时，先关闭后重新打开，再重置演示该预览自己的演示实例。即使演示不可用，也可查看静态 API 和源码。原生对话框、外部服务和操作系统集成需要单独进行桌面测试。[返回组件指南](index.md)。

## 运行细节 {#operational-details}

`BaseSchemaForm` 管理 JSON 对象模型，不负责持久保存。传入 schema、`uiHints.fields`、`fieldOptions`、`fieldErrors`，可选 `suggestionsFor` 与 `openMediaPicker`。字段 hint 的 `showIf:{key,equals,notEquals}` 控制条件显示，`advanced` 归组高级字段。所有者负责加载动态选项并保留已选值；错误以字段名为键，真实媒体选择需要提供 picker。可运行示例包括条件 textarea/media 及控件组。接受的值更新模型，保存时仍需领域/宿主验证。

`BaseFormRow` 默认只推断一个 input，`group=true` 时不自动绑定单控件。默认 slot 提供 `controlAttrs:{id,aria-describedby,aria-invalid,aria-required}`，应传给实际控件。多控件时使用 `group`，分别命名子控件，不要重复同一 id；帮助/错误关联整个组。`useSplitAttrs` 将 class/style 给外层，其余属性/监听给实际控件。文件上传只接受一个文件并检查配置的大小/类型，失败会清空模型并显示错误。选择文件并非远程上传。
