---
title: "ComboBox 与 SearchSelect"
docId: "developer.components.selects"
translationKey: "developer.components.selects"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
translationRevision: "0a29ebaa13f24f77044a4df8c7bbcd772b7ba9cb20bb3c6cfa03c716b8eb79f1"
---

# ComboBox 与 SearchSelect {#combobox-and-searchselect}

## 用途与前置条件 {#purpose}

从带标签的选项中选择稳定值。SearchSelect 同时搜索标签与值；未知的已选值会原样显示。使用现有 Vue 工作区及其 i18n 提供器。预览在隔离的应用框架中挂载真实共享组件；文档主题不会加载桌面的全局 CSS。

## 试用示例 {#example}

1. 加载预览，或打开完整演示。
2. 切换状态选择器，在适用时检查加载中、空白、错误、禁用、长文本及移动端表现。若某种状态没有独特渲染，不会为组件人为添加行为。
3. 根据控件使用 Tab、Enter、Escape 和方向键。检查无障碍名称、可见焦点，以及关闭弹出内容后焦点是否返回。
4. 修改值或调用操作，检查得到的选择或输出。刷新或重置演示会恢复演示状态。

<DemoFrame scenario="settings" locale="zh" route="/components/selects" title="ComboBox 与 SearchSelect示例" />

## API {#api}

以下 props、默认值、事件、插槽和模型声明从组件源码生成。导入的类型保留与源码的关联，不会手工复制。未声明属性遵循组件的转发契约；给包装元素绑定原生事件前，请先审查源码。

<!--@include: ../../../../.generated/components/selects.zh.md-->

## 可执行源码 {#source}

下面的同一文件既编译用于预览，也作为代码片段包含在此。少量状态属于示例本身。账户、插件和宿主逻辑应留在 features/entities 中，不应放入共享 UI。

<<< ../../../../examples/selects.vue

## 无障碍与恢复 {#accessibility}

使用可见标签或 aria-label，保留键盘操作，并避免只依赖颜色传达信息。在窄屏和较长的翻译标签下测试。对于表单，将行的 controlAttrs 绑定到实际输入控件，确保错误和必填状态仍有关联。禁用控件本身无法解释故障；应在旁边显示状态及恢复操作。

预览失败时，先关闭后重新打开，再重置演示该预览自己的演示实例。即使演示不可用，也可查看静态 API 和源码。原生对话框、外部服务和操作系统集成需要单独进行桌面测试。[返回组件指南](index.md)。

## 运行细节 {#operational-details}

加载时也应提供当前已选选项，才能显示标签；缺少选项时回退到原始模型值。`search` 发出查询供所有者选择性远程搜索，内部仍匹配标签与值。方向键/Home/End 在启用选项间移动，Enter 选择活动行。`allowCustom` 开启时，提交文本会去除首尾空白，Escape 取消编辑。除正常选择外，也测试未加载 ID、禁用行及自定义值。

## 运行细节 {#keyboard-contract}

参见[键盘、对话框例外与 tooltip 命名](../i18n-accessibility.md#operational-details)。在完整组合流程中核对初始/返回焦点，包括不可关闭对话框及已卸载的草稿编辑器。
