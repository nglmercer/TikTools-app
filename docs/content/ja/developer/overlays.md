---
title: "Widget SDK・ゲートウェイ・オーバーレイの境界"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
translationRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
order: 170
---

# Widget SDK・ゲートウェイ・オーバーレイの境界 {#widget-sdk-gateway-and-overlay-boundaries}

## 目的と前提条件 {#goal-and-prerequisites}

ローカルプレビューと OBS 出力で描画を共有するウィジェットを作ります。インストール済み Vue ツールチェーンを使い、[イベント](events.md) を理解します。ローカルプレビューはゲートウェイやアカウント認証情報が不要です。ライブ出力には別途インストールする event-gateway プラグインと、ウィジェット範囲の認証情報が必要です。

## テンプレートを追加する {#add-a-template}

1. `WidgetStage` 内に描画部を定義し、ビューポートではなく親に対してサイズを決めます。
2. `src/widgets/sdk/template.ts` の `defineWidget` にスキーマ、新しいサンプル包み、`create(search, clock)` ファクトリーを渡します。
3. `handleEnvelope`、`clear`、`dispose` と描画を実装したコントローラーを返します。渡された時計を使い、プレビューでサンプルを保持し、テストで時刻を進められるようにします。
4. `sdk/templates.ts` に登録し、単独入口、ビルド設定、ゲートウェイルート許可リスト、パッケージ項目を追加します。登録だけでは公開 URL はできません。
5. エディターと単独バンドルで同じ `WidgetHost.vue` を使います。ローカルプレビューにゲートウェイ HTML を埋め込みません。

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

テキストトークン、デザイン既定値、編集フィールドはテンプレートスキーマに置きます。デザインは JSON に直列化され、コピーした URL は検証済みスナップショットを含みます。後で保存し直した場合は新しい URL を OBS にコピーします。

## ゲートウェイのプロトコル {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` が認証、購読、ハートビート、再接続、欠落報告を担当します。ウィジェットは TikTok に直接接続せず、正規のドメイン包みを受け取ります。エンドポイントは `/ws/widgets` で、完全な外部ゲートウェイクライアント API とは異なります。共有パーサーを使い、完全なゲートウェイトークンをメイン WebView に渡さず認証境界を守ります。

## 期待される結果と提供状況 {#expected-result-and-status}

組み込みローカルプレビューと SDK は実装済みです。Cloud オーバーレイ編集と `cloud-overlay` のソースはありますが、実際の OBS 検証、画像・ライブソースの対応範囲、パッケージ適用には制限があります。オーバーレイの変更は未マージの別作業です。ランキングウィジェットがあってもランキングとコンテストは通常のナビゲーションから隠されています。

## 問題が起きたとき {#recovery}

ローカルプレビューが空ならオリジン方針を弱める前に、コントローラーの寿命、スキーマ既定値、ステージサイズを確認します。ライブ出力はゲートウェイの健全性、ウィジェット認証情報、明示的欠落を調べます。ヘッドレス Chromium の成功は OBS ブラウザーソースの受け入れを証明しません。[Widget SDK](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) と共有プロトコルテストが現行の根拠です。
