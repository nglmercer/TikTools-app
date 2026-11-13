---
title: "查找偏好设置与键盘操作"
docId: "user.preferences.index"
translationKey: "user.preferences.index"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6790ab7171a90bda1df882c0f88626a5875da68bf4f624968b0e54815842030f"
translationRevision: "6790ab7171a90bda1df882c0f88626a5875da68bf4f624968b0e54815842030f"
order: 10
---

# 查找偏好设置与键盘操作 {#find-preferences-and-keyboard-controls}

## 目标与前置条件 {#goal-and-prerequisites}

设置应用语言，找到正确的配置区域，并顺畅使用支持的键盘操作。你需要打开桌面窗口；只有配置的服务本身需要联网时，才要求网络访问。

## 查找相关设置 {#find-the-relevant-setting}

1. 打开 **设置 → 常规** 选择界面语言。当前选项为英语（`en`）、西班牙语（`es`）、葡萄牙语（`pt`）、法语（`fr`）、简体中文（`zh`）和日语（`ja`）。语言选择不会改变 TikTok 事件标识符或插件 API 名称。
2. 查看常规中的更新器控件：通道、自动检查、通知，以及支持时的手动检查。自动检查是可选的。签名更新是否可用，取决于构建已发布的签名配置。
3. 使用 **账户** 管理 Cloud 身份及诊断。使用 **协作** 配置所有者访问；宿主提供该能力时使用 **团队**。
4. 打开 **插件** 管理插件通用偏好和各插件的设置/连接面板。插件服务凭据不应填入常规设置。
5. 打开 **自动化 → 变量** 管理共用运行时值，**积分** 管理获取规则，**叠加层** 设计小组件。这些配置入口有意放在对应功能旁边。

自动安装仍属实验性，并被当前发布流程的构建门控禁用。可见的自动检查偏好并不承诺自动安装。请使用文档中的[更新流程](../updates.md)，更换版本前做好[备份](backups.md)。

## 使用支持的键盘操作 {#use-supported-keyboard-controls}

使用 Tab 和 Shift+Tab 在常规控件间移动焦点，用 Enter/空格激活已聚焦的按钮。在叠加层设计检查器中，Ctrl/Meta+Z 撤销，Ctrl/Meta+Shift+Z 或 Ctrl/Meta+Y 重做。文本输入框保留自身的撤销行为。这些是编辑器快捷键，不是全局自动化热键。

如果其他控件尚未处理 Escape，它会关闭窄屏导航抽屉。对话框和编辑器可能自行处理 Escape，包括未保存变更提示。全局热键自动化需要兼容的已安装插件及其报告的后端能力。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

这些设置和本地键盘交互均可使用，但不声称已完成全部无障碍或平台验收。优先使用有标签的按钮、易读的窗口尺寸；动态内容干扰阅读时，可暂停事件流滚动。设置似乎缺失时，先检查所属页面和宿主能力，再考虑重置数据。不可用控件见[故障排查](troubleshooting.md)，常见区别见[常见问题](faq.md)。

<DemoFrame scenario="settings" locale="zh" title="演示：设置" />
