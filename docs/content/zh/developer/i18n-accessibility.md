---
title: "国际化与无障碍"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7dc918a0278876f1490f546683b9cf084f00058f7650b72521f98bc87fbfe517"
translationRevision: "7dc918a0278876f1490f546683b9cf084f00058f7650b72521f98bc87fbfe517"
order: 120
---

# 国际化与无障碍 {#internationalization-and-accessibility}

## 目标与前置条件 {#goal-and-prerequisites}

添加在受支持语言中仍清晰可懂，并可通过键盘、触控和辅助技术使用的页面。先阅读[前端分层](frontend.md)，并使用现有共享 UI。运行时应用语言与公开文档版本是两个独立系统。

## 添加本地化内容 {#add-localized-content}

应用语言目录位于 `src/web/shared/i18n/`。英语是类型化回退语言；其他所有受支持目录均按需加载。`activateLocale` 加载选定消息并更新 `document.documentElement.lang`。通过现有语言 store 保存偏好，不要另写本地存储键。

1. 在英语目录及对应的受支持目录/模块中添加语义化消息键。
2. 保持 API ID、事件路径和存储资源标识符稳定；翻译其可见标签。
3. 变量文本使用插值，显示值使用共享国际化辅助函数。
4. 插件描述符使用现有 `{default,i18key}` 本地化文本契约及宿主合并目录。绝不要将翻译或插件提供的字符串作为 HTML 渲染。

## 保留控件语义 {#preserve-control-semantics}

优先使用 `BaseTextField`、`BaseFormRow`、`BaseDialog`、标签页和状态组件。占位文本不能替代无障碍标签。图标按钮需要本地化名称。验证必须指出字段并公开无效状态，不能只用颜色表达失败。保留对话框焦点返回、Escape 行为、键盘选择及可见焦点轮廓。

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

手动审查时，用 Tab 遍历完整流程，打开/关闭对话框，在对话框打开时切换语言，并在窄屏下重复。悬停时显示的状态操作，也必须在键盘聚焦时显示，并可通过触控使用。

## 预期结果与状态 {#expected-result-and-status}

目录类型和对等检查会捕获缺失键；共享控件提供已实现的键盘及 ARIA 行为。这不等于每个组合页面都获得全面无障碍认证。即使存在译文字符串，排名与竞赛仍保持隐藏。

## 恢复方法 {#recovery}

出现未翻译文本时，先检查延迟目录加载及精确键名，再考虑在组件中添加英语回退。焦点消失时，检查接收转发属性的控件及对话框返回目标。[i18n 实现](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts)及组件测试定义当前行为；[组件指南](components/index.md)说明共享入口。
