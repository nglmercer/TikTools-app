---
title: "安装与管理插件"
docId: "user.plugins.index"
translationKey: "user.plugins.index"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "93c8811682d0baa5bd18b571503fce483cf1f464e94b94bc933241bf277d23d4"
translationRevision: "f4d3b495b1c6d7a0ae211daf69dfb54cb180cbb39b811bb6c3d89c3049f51965"
order: 10
---

# 安装与管理插件 {#install-and-manage-plugins}

## 目标与前置条件 {#goal-and-prerequisites}

添加集成，审查其访问请求，并确认操作可用。桌面包需要兼容插件要求的操作系统、架构和宿主版本。远程目录及下载操作需要网络；部分基于账户的服务还需要[登录 Cloud](../cloud/account.md)。

## 启用前审查 {#review-before-enabling}

1. 打开 **插件**，查看已安装、探索或更新。探索仅请求插件条目。自动化模板与配置位于 [自动化](../automations/templates.md)。
2. 选择插件，检查描述、版本、兼容性及运行时。核对发布者、请求的能力或权限，不要只凭熟悉的显示名称判断。
3. 通过安装流程查看包预览。确认安装前，审查确切版本、签名信息、请求的访问权限，以及相对已安装版本的变化。
4. 等待包作业结果。下载、安装和启用是不同阶段；仅有目录卡片不代表运行时已就绪。
5. 打开插件的设置或连接面板。仅在指定字段中提供服务地址和凭据。如提供声明的连接检查，请使用它。
6. 启用插件，再检查操作类型或页面是否出现。关联功能强大的外部操作前，先创建范围有限的[日志规则](../automations/first-rule.md)。

插件可能运行原生代码或独立进程。进程边界可隔离崩溃，但不是操作系统沙箱。请安装来自可信发布者的软件。有效签名标识经过验证的包来源，但不代表每种效果都适合你的工作流程。

## 预期结果与状态 {#expected-result-and-status}

包审查、安装作业、插件偏好设置及版本管理均可使用。实际操作取决于安装的清单、宿主能力及外部服务。事件处理器有自己的面板，可为事件添加可选分析字段；原始字段仍与之区分。

## 恢复方法 {#recovery}

安装失败时，阅读作业错误，检查兼容性、签名、运行时或下载问题。不要为了消除错误而降低验证要求。更新导致问题时，检查保留版本，并在可用时回滚。禁用或卸载插件前，审查依赖它的自动化。移除插件数据与配置前，请做好[备份](../preferences/backups.md)。

<DemoFrame scenario="plugins" locale="zh" tour="plugin" title="演示：插件" />
