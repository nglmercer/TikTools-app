---
title: "TextField 与 SpinBox"
docId: "developer.components.fields"
translationKey: "developer.components.fields"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "bbf2a26d0d2b51d7eea5245dde57023be2a6a4203b1b348258c5e5206aecbbd3"
translationRevision: "03be7282c56d3f0e39f63b46ee5ebcfc8e4dff90f8898a39ec8e9d4b2eed8172"
---

# TextField 与 SpinBox {#textfield-and-spinbox}

## 用途与前置条件 {#purpose}

编辑文本和有界数值。在所属功能中执行验证，并关联标签和错误描述。使用现有 Vue 工作区及其 i18n 提供器。预览在隔离的应用框架中挂载真实共享组件；文档主题不会加载桌面的全局 CSS。

## 试用示例 {#example}

1. 加载预览，或打开完整演示。
2. 切换状态选择器，在适用时检查加载中、空白、错误、禁用、长文本及移动端表现。若某种状态没有独特渲染，不会为组件人为添加行为。
3. 根据控件使用 Tab、Enter、Escape 和方向键。检查无障碍名称、可见焦点，以及关闭弹出内容后焦点是否返回。
4. 修改值或调用操作，检查得到的选择或输出。刷新或重置演示会恢复演示状态。

<DemoFrame scenario="settings" locale="zh" route="/components/fields" title="TextField 与 SpinBox示例" />

## API {#api}

以下 props、默认值、事件、插槽和模型声明从组件源码生成。导入的类型保留与源码的关联，不会手工复制。未声明属性遵循组件的转发契约；给包装元素绑定原生事件前，请先审查源码。

<!--@include: ../../../../.generated/components/fields.zh.md-->

## 可执行源码 {#source}

下面的同一文件既编译用于预览，也作为代码片段包含在此。少量状态属于示例本身。账户、插件和宿主逻辑应留在 features/entities 中，不应放入共享 UI。

<<< ../../../../examples/fields.vue

## 无障碍与恢复 {#accessibility}

使用可见标签或 aria-label，保留键盘操作，并避免只依赖颜色传达信息。在窄屏和较长的翻译标签下测试。对于表单，将行的 controlAttrs 绑定到实际输入控件，确保错误和必填状态仍有关联。禁用控件本身无法解释故障；应在旁边显示状态及恢复操作。

预览失败时，先关闭后重新打开，再重置演示该预览自己的演示实例。即使演示不可用，也可查看静态 API 和源码。原生对话框、外部服务和操作系统集成需要单独进行桌面测试。[返回组件指南](index.md)。
