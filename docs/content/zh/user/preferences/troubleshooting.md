---
title: "诊断失败的操作"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
translationRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
order: 30
---

# 诊断失败的操作 {#diagnose-a-failed-operation}

## 目标与前置条件 {#goal-and-prerequisites}

定位故障组件，收集有用报告，并避免反复触发效果。你需要应用版本、故障大致时间及尝试的操作。重置或替换数据前，请保留[备份](backups.md)。

## 查看最直接的证据 {#inspect-the-nearest-evidence}

1. 阅读故障位置的行内错误。使用其诊断状态操作，或打开 **诊断** 查看相关条目。
2. 按来源、级别或文本筛选。有用的来源包括 LIVE、监控、插件、账户、同步、更新、传输、目录、安装与协作。
3. 选择相关条目，保留时间、代码、请求/实体标识符及简短说明。有详情时展开查看。
4. 复制筛选后的诊断信息或导出文本报告。剪贴板/导出失败时，使用显示的备用文本。分享前检查并移除私有令牌、Cookie、邀请链接及个人内容。
5. 提供简短复现步骤：页面、所选主播/配置/插件、执行操作、预期结果和实际结果。说明发生在更新前还是更新后。

## 隔离常见故障 {#isolate-common-failures}

事件流为空时，清除搜索与筛选、恢复滚动并确认[连接](../live/connections.md)。暂停滚动不等于断线。显示丢失提示不能证明积分或统计也丢失了同一事件。

自动化没有运行记录时，检查当前配置、事件/操作启用状态、触发器、筛选字段路径及冷却时间。改变效果前先查看运行历史。**运行测试** 可能执行音频、HTTP、积分及插件操作。

插件故障请阅读安装作业或连接错误。Cloud 故障请检查账户状态及受影响区域，不要更换 TikTok Cookie。叠加层请检查网关状态和源配置。更新无法安装时，请使用[手动更新指南](../updates.md)。

## 预期结果与状态 {#expected-result-and-status}

诊断筛选、复制及文本导出均可使用。打包应用还需要其 Web 资源和平台运行时。Linux 上检查受支持的 GTK/WebKitGTK 环境及 X11/XWayland 会话；Windows 上检查 WebView2。安装范围见[系统要求](../requirements.md)。

## 恢复方法 {#recovery}

修正具体原因后再重试。修订冲突时，重新加载并审查当前保存状态。远程命令未确认时，先检查日志/状态再发送另一个命令：超时也可能发生在效果已成功之后。启动失败时，报告安装包、版本及启动日志，不要直接删除数据目录。

<DemoFrame scenario="diagnostics" locale="zh" tour="diagnostics" title="演示：诊断" />
