---
title: "插件打包、签名与注册表"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
translationRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
order: 280
---

# 插件打包、签名与注册表 {#plugin-packaging-signing-and-registries}

## 目标与前置条件 {#goal-and-prerequisites}

分发可独立检查字节、发布者权限和宿主兼容性的包。从已验证插件和受支持目标开始。签名需要保存在仓库之外的作者私钥；注册表根委托是独立的运维职责。

## 准备可审查的归档 {#prepare-a-reviewable-archive}

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` 编译包并创建 `.plugin` 归档。清单入口、资源、语言文件、UI 输出及相关运行时依赖，连同 `checksums.json` 一起打包。native/process 构建需要兼容的目标产物；纯 JavaScript 包保持可移植，直到原生扩展或平台设施对其施加限制。

对已有构建归档，请使用确切文件路径，不要使用未经审查的通配符：

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

将归档名替换为本次构建实际生成的目标特定输出。签名覆盖校验和，校验和覆盖包文件。公钥可以分享；私钥文件不得进入版本控制或日志。

## 信任与更新策略 {#trust-and-update-policy}

静态注册表协议具有根签名权限及委托的索引/发布者密钥。发布者命名空间批准不等于包完整性，Cloud 目录摘要也不能替代任何一条签名链。解析器检查通道、引擎、schema/协议/ABI、运行时及目标。即使文件哈希匹配，已撤回、已吊销或不兼容发行版也不具备资格。

宿主安装前先显示审查。自动更新要求策略允许、包未固定、签名发布者兼容且没有新增权限；扩大权限必须审查。产品 Cloud 目录安装不同于底层静态注册表编写，常规产品流程拒绝任意远程包 URL。

## 预期结果、恢复与验证 {#expected-result-recovery-and-verification}

安装器可在应用已审查包之前验证字节，并保留回滚/恢复状态。开发暂存不属于安装器管理。信任失败时，修复委托或发布者元数据，不要将未签名覆盖标志作为常规分发策略。运行时启动不确定时，停止替换并协调确认保留的结果。

使用 `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` 和注册表测试验证离线夹具。[包验证器](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src)及[注册表模型](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs)负责技术格式；发布基础设施需要单独运维验收。
