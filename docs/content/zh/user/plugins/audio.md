---
title: "设置声音与语音"
docId: "user.plugins.audio"
translationKey: "user.plugins.audio"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b6be887705d13dfe120f3e07dc4006ec0efa1472b6dbe0ccfc1ebfc1e83f3672"
translationRevision: "b49970a3fcfb4936d7f011f21f3622452df87f0f84361fc6d38aa0e2e086d0f6"
order: 20
---

# 设置声音与语音 {#set-up-sounds-and-speech}

## 目标与前置条件 {#goal-and-prerequisites}

先验证可预测的声音播放，再把音频关联到繁忙的直播活动。你需要正常的输出设备和现有且受支持的音频文件。本地 **播放声音**（`audio.play`）是内置操作；已安装的 SonicBoom 集成等语音提供商需要单独配置插件或服务。

## 测试本地声音 {#test-a-local-sound}

1. 在 **自动化 → 操作** 中创建 **播放声音**（`audio.play`）。使用媒体选择器选择已有文件。
2. 首次检查将音量设为 `0.2`。支持范围为 `0` 到 `1`，零表示静音。
3. 选择重叠策略：`allow` 允许同时播放，`restart` 重新开始播放，`drop` 在当前声音播放时忽略新的实例。
4. 保存操作，并在选择 **运行测试** 前阅读配置。此测试可能播放可听见的声音。
5. 通过预期设备试听。本地播放成功后，将操作关联到带有保守[筛选与冷却](../automations/recipes.md)的事件。

该操作保留基于路径的媒体引用。TikTools 不会将音频文件复制到数据库。请将文件保留在该路径，并在迁移或备份环境时单独包含它。

## 配置可选语音 {#configure-optional-speech}

通过[插件](index.md)安装可信的语音插件。在其连接或设置控件中，验证服务地址、身份验证、所选提供商、语言及声音/配置。测试简短且不敏感的句子前，先加载声音目录。缺少声音时，应排查目录、所有权或提供商是否不匹配，不要编造标识符来替代。

对 SonicBoom 而言，合成、获准播放及可听播放完成是不同结果。已排队或已开始的响应不能证明有人听到了句子。克隆语音可能慢于聊天到达速度，因此请查看队列或饱和诊断，并限制事件准入数量。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

宿主声明支持音频时，可使用本地播放。语音仍依赖已安装插件和可访问服务；这里不保证提供商、设备或平台已通过验收。无声时，检查文件是否存在、音量、输出设备、暂停状态及运行错误。语音失败时，先检查服务和声音目录再重试。直播期间避免反复测试效果；使用[诊断](../preferences/troubleshooting.md)时不要分享令牌或朗读的私密文本。
