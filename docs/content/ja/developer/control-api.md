---
title: "Control API の契約と機能検出"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
translationRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
order: 70
---

# Control API の契約と機能検出 {#control-api-contracts-and-discovery}

## 目的と前提条件 {#goal-and-prerequisites}

メソッド名や応答スキーマを作り上げず、ホストの公開契約を呼びます。非公開の起動済みホストと対応 CLI を使います。正規の [Rust レジストリ](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) がメソッドを記述し、登録モジュールがスキーマ、機能、デスクトップ要件、並行性フラグを提供します。操作を選ぶ前に実際のビルドを検出します。

## 調査してから呼ぶ {#inspect-then-call}

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

最後のコマンドは分離された検証ランタイムを使い、実際の LIVE 接続は検証しません。ローカル stdio/IPC は改行区切り JSON、リモート WSS は1フレーム1メッセージで、先に `collaboration.hello` が必要です。

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

応答は ID を返し、`result` または `error` を含みます。エラーは文字列 `code`、メッセージ、任意の安全な `data` を使います。無関係な JSON-RPC サーバーの数値コードを前提にしないでください。通知は `method:"event"` と `params:{topic,data}` を使います。通知と保留応答の解決は分離します。

## メタデータを保持する {#preserve-metadata}

バージョン付き呼び出しは最上位に `meta.expectedRevision` を持てます。永続呼び出しは機能検出に従いコマンド ID とホストインスタンスも結び付けます。ドメイン結果とともに `meta.resourceVersions`、`meta.command`、`meta.hostInstanceId` を読みます。リビジョンは10進文字列で、JavaScript の数値ではありません。`src/web/shared/api/control-contracts.ts` の `callWithMeta` がこの包みを保持します。

## 期待される結果と提供状況 {#expected-result-and-status}

検出されたメソッドはそのホストビルドが対応しますが、リモート権限とリソース制限は別途適用されます。ローカル所有者用メソッドは自動的にリモート用にはなりません。汎用要求も型付き SDK と同じ操作経路を使います。共有技術資料は別の手作業表ではなく、レジストリ・スキーマから導出します。

## 問題が起きたとき {#recovery}

`method_not_found` は検出結果とクライアント・ホスト版、`revision_conflict` は候補を保持して現行版を比較します。通信タイムアウト後も受け付けた変更が続く場合があります。対応していれば保持した元のコマンド ID を照会します。不確実さを迂回するため新しい ID を作らないでください。[エージェント](agents.md) または[リモート復旧](remote-client.md) に進みます。

## バージョン付きメソッドリファレンス {#method-reference}

コミット済みの機能検出スナップショットから生成しています。実行中のホストが最終的な根拠です。

<!--@include: ../../../.generated/control-api.ja.md-->
