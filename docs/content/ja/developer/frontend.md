---
title: "フロントエンドの層とデータの流れ"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
translationRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
order: 40
---

# フロントエンドの層とデータの流れ {#frontend-layers-and-data-flow}

## 目的と前提条件 {#goal-and-prerequisites}

ページをネイティブ IPC と密結合させず Vue アプリを拡張します。インストール済みワークスペースの範囲から Vue 3.5、Pinia 4、Vue Router 5、vue-i18n 11 を使います。新しいひな型を作らず、ロック済み依存関係と[セットアップ](setup.md) のコマンドを使います。

## 担当する層を見つける {#find-the-owning-layer}

現在の Feature-Sliced Design 構成は `src/web/` にあります。

| 層 | 責任 | 既存の例 |
| --- | --- | --- |
| `app` | 起動、ルーティング、シェル、実行時の寿命 | `app/model/use-app-runtime.ts` |
| `pages` | ルートの構成 | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | 大きな複合パネル | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | ユーザー操作と下書き | `features/apply-rule-template/` |
| `entities` | ドメインストアと再利用ドメインロジック | `entities/rule-template/` |
| `shared` | API、基本 UI、i18n、汎用処理 | `shared/api/control-client.ts` |

上位層は下位層から組み立てます。ホスト呼び出しは共有制御クライアントを介し、通信選択は起動処理が担当します。古い文書の `src/web/platform` と `src/web/views` は過去の配置です。

## 範囲を絞って変更する {#implement-a-focused-change}

1. `app/router/routes.ts` のルートからページ、feature、entity ストアをたどります。
2. 入力、状態、ダイアログに既存の共有部品を使い、ラベル、フォーカス動作、イベント契約を維持します。
3. 編集候補と保存済みホストレコードを分離します。`shared/lib/request-scope.ts` で古い非同期応答を無視します。無効化しても受け付け済みホスト変更はキャンセルされません。
4. `shared/i18n` に翻訳キーを加え、所有側の寿命で購読を破棄します。

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## 期待される結果・提供状況と対処 {#expected-result-status-and-recovery}

ルートは現在のホストデータを描画し、競合時も未保存編集を保持します。ランキングとコンテストはソースとルートがあっても通常のナビゲーションから隠されています。部品の存在だけで製品上の利用可否を推測しません。ネイティブ、ローカルブラウザー、デモの起動は同じ機能コードに異なる通信を提供できます。

古い応答が新しい照会を置き換える場合は、バックエンド変更前に世代を確認します。プラグインページが失われたら、実装どおりインストール済み一覧に戻ります。接続、再同期、終了処理の根拠は[アプリケーションランタイム](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) です。
