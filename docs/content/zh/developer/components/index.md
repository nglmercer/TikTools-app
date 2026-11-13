---
title: "共享 UI 组件与组合方式"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
translationRevision: "4c70fe5256ba711f6f8fd6c3e8d305c4a44bdd6ba06092a846015e6f1898744f"
order: 290
---

# 共享 UI 组件与组合方式 {#shared-ui-components-and-composition}

## 目标与前置条件 {#goal-and-prerequisites}

使用现有 UI 工具包组合功能，并保留交互及本地化行为。使用 Vue 工作区已安装的工具链，先阅读[前端结构](../frontend.md)。本页介绍工具包；各组件的源码、props、测试及专属组件族参考，仍是其确切 API 的权威依据。

## 找到合适的基础组件 {#find-the-correct-primitive}

`src/web/shared/ui/` 包含按钮、文本/日期/颜色字段、文本区域、复选框、开关、组合框、标签页、表格、表单、对话框、状态、警告及结构控件。设计令牌位于 `src/web/app/styles/tokens.css`；基础与响应式样式共同组成应用。间距、颜色、字体及焦点样式请使用令牌，不要在每个功能中另建主题。

1. 找到已有的、具备所需交互的功能。
2. 包装基础组件前，阅读其 `defineProps`、插槽、发出值/模型值及测试。
3. 将领域验证、保存状态和宿主调用留在所属的 feature/entity 中。
4. 按基础组件支持的转发契约传递无障碍标签和控件属性。

script-setup 组件可使用真实的按钮 API：

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
</script>

<template>
  <BaseButton variant="primary" type="submit">{{ t('common.apply') }}</BaseButton>
</template>
```

包含它的表单负责提交行为及验证。`BaseButton` 支持 `default`、`primary`、`danger` 和 `ghost` 变体；破坏性颜色本身并不构成确认流程。

## 预期结果与状态 {#expected-result-and-status}

共享基础组件提供一致且已实现的键盘、焦点和 ARIA 行为。组合后仍需审查：占位文本不是标签，仅有图标的控件需要名称，禁用状态应反映当前操作的归属，异步错误必须保持可见。开发 UI 工具包路由是演示器，不是独立产品功能或后端验收测试。

## 验证与恢复 {#verify-and-recover}

运行 `bun scripts/task.ts test web BaseButton`、所改组件的针对性测试，以及 `bun scripts/task.ts frontend-lint`。检查窄屏、键盘导航和翻译后的标签。属性落到包装元素而非输入框时，先查看 `shared/lib/use-split-attrs.ts` 和控件测试，再考虑绕过基础组件。调整组件族参考示例时，请查阅[无障碍指南](../i18n-accessibility.md)及 [UI 源码](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui)。

## 交互式组件族参考 {#families}

- [按钮与操作](buttons.md)
- [菜单](menus.md)
- [TextField 与 SpinBox](fields.md)
- [ComboBox 与 SearchSelect](selects.md)
- [开关](switches.md)
- [表单](forms.md)
- [标签页与工具栏](tabs.md)
- [列表与表格](collections.md)
- [筛选与选择](filters.md)
- [警告、空状态与状态提示](feedback.md)
- [面板与对话框](panels.md)
- [FloatingPanel 浮动面板](floating.md)
- [Splitter 与 MasterDetail](layouts.md)
- [JSON 查看与差异比较](json.md)
- [导入与审查](import-review.md)
- [Cloud 审查模式](cloud-patterns.md)
- [叠加层模式](overlay-patterns.md)
