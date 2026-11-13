---
title: "备份与恢复本地工作"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: "zh"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
translationRevision: "7e6d88e43b32698c36afd6385bee8c44cc1223d669dd4c95414db604c17f254f"
order: 20
---

# 备份与恢复本地工作 {#back-up-and-restore-local-work}

## 目标与前置条件 {#goal-and-prerequisites}

在更新、重置或更换电脑前保留正常设置。你需要自己控制的存储空间，并留出时间正常关闭 TikTools。当前界面没有通用的 **备份所有内容** 按钮；请结合专用导出与关闭应用后的数据复制。

## 导出可复用的定义 {#export-reusable-definitions}

1. 在 **自动化** 中选择目标配置，用导出操作保存 `.tikprofile.json` 文件。文件名中注明应用版本与日期。
2. 单个可复用模板，可在模板对话框提供的下载/导出入口保存为 `.tiktemplate.json`。
3. 分享前检查导出文件。模板可能包含目标地址及配置值。配置导出不包含余额、统计、运行时变量、媒体文件或账户凭据。

## 复制本地数据 {#copy-the-local-data}

1. 停止直播活动和协作服务，然后正常退出 TikTools。等待宿主停止写入。
2. 找到当前数据根目录。默认 Windows 为 `%LOCALAPPDATA%\TikTools`，Linux 为 `~/.local/share/TikTools`；配置了 `XDG_DATA_HOME` 时，则位于其下的 `TikTools`。自定义 `TIKTOOLS_HOME` 或各目录覆盖项会改变位置。
3. 将根目录及所有单独覆盖的数据、插件、插件数据和插件存储目录复制到带日期的备份文件夹。复制已关闭数据库的完整目录，包括现存 SQLite 附属文件，不要只挑选一个 `.db` 文件。
4. 单独保存引用的音频文件：音频操作保存路径，不导入声音字节。将应用版本和自定义目录位置与备份一同记录。
5. 保护该副本，因为本地配置可能含有私有值。Cloud 身份验证保存在操作系统凭据存储中，不会随此文件夹复制转移。

## 恢复与验证 {#restore-and-verify}

关闭 TikTools 后，先将当前文件夹另存为恢复副本。将备份文件恢复到预期路径，再启动兼容版本。重新连接前，检查配置选择、余额、变量及媒体路径。便携式定义请通过自动化导入导出的配置或模板；导入定义会保持禁用，直到明确激活。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

关闭应用后的复制，以及这里说明的导入/导出入口，都是可用的恢复方法，但不保证任意版本之间都能恢复。Cloud 更改不是完整备份。如果恢复后的版本拒绝数据，请停止并回到保留副本；重新安装旧二进制文件不会撤销数据库迁移。请参阅[更新](../updates.md)和[故障排查](troubleshooting.md)。
