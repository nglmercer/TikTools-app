---
title: "构建第一个插件"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
translationRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
order: 250
---

# 构建第一个插件 {#build-a-first-plugin}

## 目标与前置条件 {#goal-and-prerequisites}

生成经过验证、只返回无害操作摘要的包。使用与仓库匹配的 `tiktools-plugin` devkit，TypeScript 开发使用 Bun 1.4.4。在独立开发目录中创建示例；这是作者开发示例，不是应用依赖变更。

## 创建脚手架并验证 {#scaffold-and-validate}

在 TikTools 检出目录中，通过任务运行器构建作者 CLI：

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

将该构建的 `target/debug/tiktools-plugin` 加入工具路径，然后在选定的开发目录中运行：

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

脚手架创建 schema-v3 清单、带发布者前缀的稳定 ID、操作描述符、SDK 导入及构建配置。保持运行时 `napi-vm`、`apiVersion:1` 和构建模块入口一致。该名称是历史遗留，当前执行使用 QuickJS。

## 编写并构建客体代码 {#write-and-build-the-guest}

生成的操作可完全保持在类型化结果边界内：

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? '你好'));
  },
});
```

在该独立包中安装其声明的依赖，再运行 `tiktools-plugin build --release`。它提前编译 JavaScript 并打包归档，不存在运行时 TypeScript 转译器或 Node 进程。[TypeScript SDK](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts)负责辅助函数签名；清单 schema 提供编辑器辅助，宿主解析器仍是权威。

## 预期结果与状态 {#expected-result-and-status}

`validate` 报告兼容性及发布问题；构建成功会创建包含清单、入口及完整性元数据的 `.plugin` 包。仅返回摘要的操作不需要额外宿主效果。添加事件发出、HTTP 或音频时，先更新声明并审查所请求的权限，再进行测试。

## 恢复与验证 {#recovery-and-verification}

入口缺失时，先构建再打包，并检查相对于清单的路径。不要将 process 运行时指向未编译的 JavaScript。本地暂存使用 `tiktools-plugin dev stage --into /absolute/path/dev-plugins`，并使用配置了该根目录的私有测试宿主。开发暂存与安装不同，不能证明签名注册表信任成立。分发示例前，请继续阅读[运行时限制](runtimes.md)与[签名](packaging-registry.md)。
