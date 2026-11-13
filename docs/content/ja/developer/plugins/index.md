---
title: "プラグイン開発の概要"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "3005a725ee4752a5afa7ed48b02b426d806faf92442c62266cbcf9dcf1457eef"
order: 240
---

# プラグイン開発の概要 {#plugin-development-overview}

## 目的と前提条件 {#goal-and-prerequisites}

目的の連携を実現する最小のプラグイン境界を選びます。TikTools は組み込み、ユーザー、開発用ルートから実行パッケージを検出します。パッケージ追加のため特定プラグインをホストへ再コンパイルする必要はありません。対応する devkit/SDK と非公開の開発プロファイルで実処理を試します。

## 作成の流れ {#follow-the-authoring-path}

1. [クイックスタート](quickstart.md) で分離パッケージのひな型を作り、検証・ビルドします。
2. 拡張子だけでなく、ネイティブ依存と信頼に基づき[ランタイム](runtimes.md) を選びます。
3. ホストが処理を確認し設定を描画できるよう[権限と UI](permissions-ui.md) を宣言します。
4. 完全性、発行者の信頼、更新方針には[パッケージとレジストリ](packaging-registry.md) を使います。

低水準の Rust の正本は `crates/tiktools-plugin-api` で、マニフェスト検証、プロトコル、UI 契約を持ちます。`crates/tiktools-plugin-sdk` は Rust 作成 API、`packages/plugin-sdk` は TypeScript ヘルパーと[エディタースキーマ](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) を提供します。読み込みはフロントエンドではなく `crates/tiktools-plugin-loader` が担当します。

## 提供できる機能 {#understand-contributions}

アクションは型付き結果を通して副作用を要求します。イベントソースはトリガーを公開し、オブザーバーは安定した `{topic,data}` ドメイン包みを上限付きで受信します。プロセッサーは期限、決定的な結合、健全性の分離の下で、副作用のない証拠を `event.intel` に追加します。宣言的な設定・アクションはホストが通常の設定画面を描画できます。独自のコンパイル済みページは専用フレーム・ウィンドウ内の制限付きブローカーを使い、スクリプトはメイン Vue ランタイムに入りません。

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

これはホスト・SDK の対象検査で、任意の外部パッケージを検証するものではありません。

## 期待される結果と提供状況 {#expected-result-and-status}

native、process、declarative、QuickJS ベースの `napi-vm` は実装済みです。WASM は任意の枠で、通常ビルドに汎用エンジンはありません。連携例は別の [plugins-example-tiktools リポジトリ](https://github.com/nglmercer/plugins-example-tiktools) にあります。旧文書のソース名だけで同梱製品機能とは判断できません。

## 問題が起きたとき {#recovery}

検出失敗ではマニフェスト互換性、ルート優先順位、対象、入口パスを確認します。要求が拒否されたらホストを迂回せず宣言済み・付与済み機能を確認します。結果不明の起動は置換前に照合します。native/process は OS レベルの信頼制限を持ち、ランタイム名だけではサンドボックスになりません。
