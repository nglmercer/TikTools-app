---
title: "Control API の契約と機能検出"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "11fa6a4eda1c79e97febd02d593c9357cf119ca5d7b0111fe112b40ecd65d381"
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

## 運用の詳細 {#operational-details}

使い捨て `TIKTOOLS_HOME` を使い、[ストレージ](storage.md)の継承パス設定を解除し、`tiktools host --ipc` を起動します。制御端末も同じ home を使います。例は使い捨て workflow `docs-example` がある前提なので、先に[自動化](automations.md#operational-details)の無効な例を作ります。読み取りの `meta.resourceVersions` の版と `meta.hostInstanceId` を保存し、`REVISION`・`HOST_INSTANCE` を置換します。同じコマンド ID と正確な payload を保持し、timeout 後は元 ID を照会してリソースを再取得します。`accepted`・`running`・`unknown` は新しい副作用を許可しません。版競合では読み直して再レビューし、自動上書きしません。`points.adjust` は加算型でコマンド ID は受け付けますが expected revision は受け付けません。

`--include-meta` は結果とメタデータを残し、通常出力は結果だけを抽出します。`--dry-run --explain` は実行せず検証・計画し、説明は stderr、JSON は stdout です。破壊的呼び出しには `--confirm` が必要です。既知の秘密は既定で秘匿し、`--secrets-visible` もホストが隠した値は復元できません。手作業で確認します。`api events` は時間・件数・topic を制限します。features・権限は実ホスト discovery が正で、生成する契約説明はレジストリ原文を保持します。

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
