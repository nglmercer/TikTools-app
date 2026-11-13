---
title: "アプリケーションとサービスの構造"
docId: "developer.architecture"
translationKey: "developer.architecture"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
translationRevision: "9726c146c879432a9964d4eaed6e35b3645100b390f6e08e87b67b2a4e854bb8"
order: 30
---

# アプリケーションとサービスの構造 {#application-and-service-architecture}

## 目的と前提条件 {#goal-and-prerequisites}

状態を所有するサービスに変更を配置します。[セットアップ](setup.md) を読み、既存ホストと明示的な単独ホストの違いを理解します。デスクトップは `AppCore` を1つ作り、1つの `ControlApi` で包み、そのインスタンスを WebView、ローカル CLI、エージェントに公開します。

## 操作の経路を追う {#trace-an-operation}

1. `crates/tiktools-control-api/src/modules/` でメソッド登録を探します。登録はスキーマ、方針、検出メタデータを提供します。
2. ハンドラーから `crates/tiktools-core/src/control/` と担当サービスへ進みます。ドメイン検証と永続的な変更は通信アダプターではなくここに置きます。
3. `crates/tiktools-client` に型付きラッパーを追加します。整合性テストが SDK の対応範囲と登録済み API を比較します。
4. `src/web/shared/api/control-client.ts` 経由で利用します。Vue の entities が表示状態を持ち、デスクトップは WebView に DB やネイティブ連携オブジェクトを渡しません。

Winit はイベントループ、Wry は WebView、トレイ連携はトレイを所有します。`HostEmitter` とドメインイベントが直列化した値をコアから運びます。デスクトップはイベントループプロキシを介して処理を渡し、WebView のスクリプト評価はそのスレッドで行います。非同期ホスト処理は Tokio が実行します。

## 具体的な境界の確認 {#concrete-boundary-check}

```sh
bun scripts/task.ts test web control-client
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
```

最初はフロントエンドクライアントを検査します。次はロックファイルを保ったまま SDK の統合テスト対象を直接実行します。Cargo の対象フラグは狭い対象テスト構文ではなく `rust` の引き渡しを使います。

## 期待される結果と提供状況 {#expected-result-and-status}

全クライアントが同じ権威ある操作・イベント経路を観測します。`host --ipc`、`host --stdio`、`--standalone` は意図的にヘッドレスの所有権を作り、通常の CLI は既存の所有ホストへ接続します。旧 `PageMessage`/`IpcRouter` は互換コードとして残り、新しいメッセージ種別を追加すべきではありません。

## 問題が起きたとき {#recovery}

クライアント間で食い違う場合は、別ストアを追加する前にホスト ID、ホーム、ビルド機能、検出結果を比べます。プラグイン不足は実行時カタログの問題で、コアへ実装を組み込む理由にはなりません。過去の構造文書には廃止されたフロントエンドパスがあります。異なる場合は[現在のコア](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/lib.rs) と[フロントエンド構成](frontend.md) に従います。

![TikTools の AppCore、通信層、ControlClient、実際の Vue ワークスペース](/architecture.svg)
