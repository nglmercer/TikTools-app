---
title: "最初のプラグインを作る"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
translationRevision: "61201a958231d15db3c9d5b2aad72fa33b1863224ebb395b7e7a879aa1eb16e8"
order: 250
---

# 最初のプラグインを作る {#build-a-first-plugin}

## 目的と前提条件 {#goal-and-prerequisites}

無害なアクション要約を返す検証済みパッケージを作ります。リポジトリに対応する `tiktools-plugin` devkit と、TypeScript 用 Bun 1.4.4 を使います。例は別の開発ディレクトリに作成します。作成方法の例であり、アプリの依存関係変更ではありません。

## ひな型を作り検証する {#scaffold-and-validate}

TikTools のチェックアウトでタスクランナーから作成用 CLI をビルドします。

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

出力の `target/debug/tiktools-plugin` をツールパスへ追加し、選んだ作成ディレクトリで次を実行します。

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

ひな型には schema-v3 マニフェスト、安定した発行者接頭辞付き ID、アクション記述子、SDK インポート、ビルド設定が含まれます。`napi-vm`、`apiVersion:1`、ビルド済み入口を揃えます。名前は歴史的なもので、現在の実行は QuickJS です。

## ゲストを記述しビルドする {#write-and-build-the-guest}

生成アクションは型付き結果の境界内だけで完結できます。

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? 'こんにちは'));
  },
});
```

その別パッケージで宣言済み依存を導入して `tiktools-plugin build --release` を実行します。JavaScript を事前コンパイルしてアーカイブにします。実行時 TypeScript 変換や Node プロセスはありません。[TypeScript SDK](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts) がヘルパーのシグネチャを所有します。マニフェストスキーマは編集補助で、ホストパーサーが最終的な正本です。

## 期待される結果と提供状況 {#expected-result-and-status}

`validate` は互換性と公開上の問題を報告します。成功したビルドはマニフェスト、入口、完全性情報を含む `.plugin` を作ります。要約だけのアクションに追加ホスト処理は不要です。イベント送出、HTTP、音声を追加する場合は、宣言を更新して要求権限を確認してから試します。

## 復旧と検証 {#recovery-and-verification}

入口がなければ梱包前にビルドし、マニフェストからの相対パスを確認します。process ランタイムに未コンパイル JavaScript を指定しません。ローカル配置は `tiktools-plugin dev stage --into /absolute/path/dev-plugins` と、そのルートを設定した非公開ホストを使います。開発配置とインストールは別で、署名レジストリの信頼は確認できません。配布前に[ランタイム制限](runtimes.md) と[署名](packaging-registry.md) に進みます。
