---
title: "导入模板与管理配置"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e9847c93a8a75fd5a58cd115b49b8e0bcb091ceda1dbdf2c226b52667eb17e23"
translationRevision: "82274eaa2e4b3162b5db46f9f2d805a14b92572a8fe8fe64bfe985371a495dda"
order: 50
---

# 导入模板与管理配置 {#import-templates-and-manage-profiles}

## 目标与前置条件 {#goal-and-prerequisites}

复用已审查的单条规则或一组规则，同时避免意外激活效果。你需要 **自动化**、可信的 JSON 文件或目录条目，以及其中操作所需的插件。替换既有设置前，请导出当前配置。

## 选择正确的入口 {#choose-the-right-surface}

`.tiktemplate.json` 文档描述一个事件及其操作，`.tikprofile.json` 文档则将多个模板归组。在自动化中打开模板对话框，可使用内置或自定义模板、Cloud 模板库或手动模板导入。使用配置工具栏创建、选择、导出配置或手动导入配置。**插件 → 探索** 浏览的是插件；模板与配置应在此处管理。

## 审查与导入 {#review-and-import}

1. 打开对应的导入对话框，选择文件或粘贴支持的 JSON。继续前先核对暂存的名称与来源。
2. 模板需检查触发器、操作类型和参数；配置还需检查条目数量及共有参数。
3. 将示例目标、媒体路径及观众标识符替换为适合自己环境的值。不要将凭据粘贴到打算分享的文件中。
4. 明确执行应用或导入，并等待结果。不要假定导入定义处于禁用状态，应检查审核中的 enabled 值。随后选择它们，逐一审查操作和事件，再明确启用需要的项目。
5. 先进行仅写日志的测试，或先检查定义。HTTP、音频、积分或插件测试可能产生实际效果。
6. 确认切换前，审核目标成员将启用、其余记录将禁用，包括独立记录及原先禁用的目标成员。副作用仍需触发器/条件；这不是 Cloud 同步。

<code v-pre>{{ params.giftName }}</code> 等参数会在应用模板时解析。<code v-pre>{{ event.user.uniqueId }}</code> 等运行时引用保留到每个事件发生时处理，而[全局变量](variables.md)在执行时提供共用的已保存值。

## 预期结果与状态 {#expected-result-and-status}

本地模板和配置的导入、导出入口均可使用。浏览 Cloud 需要对应的宿主能力及账户或服务。目录预览不能证明操作的外部服务正常。缺少插件时，导入定义可能在完成安装配置前无法使用。

## 恢复方法 {#recovery}

JSON 无效时，请依据报告的解析错误及预期文档类型修正；重命名任意 JSON 文件并不能转换其格式。修订冲突时，请先刷新已保存的规则库再应用。保留导出的正常配置；配置导出不包含哪些数据，请参阅[备份](../preferences/backups.md)。

## 运行细节 {#operational-details}

手动模板导入先保存目录文档；应用模板才创建关联动作/事件。解析器在缺少 `enabled` 时默认 true，应用流程保留这些值。手动配置导入也将条目交给所有者审核。Cloud 目录模板导入保存模板；Cloud 配置导入携带来源信息准备配置变更。不能因来源或预览成功就断言全部禁用。检查最终比较及已保存状态。CLI 导入应明确使用 `tiktools template import file.tiktemplate.json --disabled` 或 `tiktools template profile-import file.tikprofile.json --disabled`。

切换配置会改变启用状态：确认后启用目标成员，禁用所有非成员动作/事件，包括独立记录。目标中原先禁用的成员可能重新启用。确认前检查两类清单。实际副作用仍要求匹配触发器、条件及可用运行时；切换本身不发送 Cloud 同步。
