---
title: "リモートクライアントと復旧"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3fa6b05b6c19548fa64d45b3114bfd8080f3f0ed510f835c4d18297d9ccd6cae"
translationRevision: "c3a61318016c883c9bcd75e3561ec7e75cc1812bf57e0ffe0033579f7af3ebcf"
order: 90
---

# リモートクライアントと復旧 {#remote-clients-and-recovery}

## 目的と前提条件 {#goal-and-prerequisites}

ローカルに LIVE コアを作らずコントローラーをホストへ接続します。必要なリモート機能が有効なホスト、所有者承認済み招待、到達可能な HTTPS オリジン、対応する安全な認証情報ストアを用意します。付与権限は[共同操作](collaboration.md)、ネイティブ経路は[通信](transports.md) を読みます。

## ペアリングと調査 {#pair-and-inspect}

以下は手動テスト環境の例です。例のオリジンを設定したホストに置き換えます。

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

自己署名ホストでは別経路で所有者と指紋を照合し、ペアリング時に `--fingerprint` を渡します。inspect は認証付き業務要求を送らず、信頼も承認しません。招待・認証情報はコマンドラインに秘密を置かず専用プロンプトで入力します。

プロファイルには ID、信頼方針、安全なストアへの不透明参照を保存します。Linux は Secret Service、macOS は Keychain、Windows は Credential Manager を使います。ストアがなければ明示的に失敗し、Android は永続化バックエンドが未選択です。SDK は明示的なメモリー内認証情報を使えます。`remote forget` はローカル情報を消しますが、ホストのセッションは取り消しません。

## コマンドの同一性を保つ {#preserve-command-identity}

対応する変更の前にメタデータ付きでリソースを読みます。基準リビジョン、ホストインスタンス、永続コマンド ID を保持します。送信後に切断したら元の ID で `collaboration.commands.get` を照会します。受領記録は過去の情報なので現行リソースは別途更新します。ワークフロー無効化・削除、ポイント調整、無効な作成者所有の自動化下書きは契約と権限が異なります。

## 期待される結果と提供状況 {#expected-result-and-status}

ネイティブデスクトップには分離した `tiktools-desktop --controller` 入口とプロファイルに結び付くジャーナルがあります。対応範囲は実験段階です。ペアリング、通信接続、広い機能フラグだけでは任意編集を許可しません。リモート自動化読み取りは編集可能なソースではなく限定要約です。リモート enable/fire とローカル所有者の採用は別です。

## 問題が起きたとき {#recover}

シーケンス欠落では交渉済み購読の snapshot/resume を使い、ID 変更時は非公開投影を消します。再接続後に業務変更を自動再実行しません。証明書変更には明示的な信頼確認が必要です。[プロファイル実装](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs) にリースと更新の不確実性があります。実際の複数 PC・OS ストアの受け入れはループバックテストと別に報告します。

## 運用の詳細 {#operational-details}

信頼イベントは対応するスナップショット・投影へ適用してから `acknowledge` します。先に確認すると回復証拠を失います。再開にはホストインスタンス・購読・セッション・grant の一致が必要です。gap、文脈変更、カーソル拒否、gap によるブラウザー切断では新しい認可済みスナップショットと購読を取得します。履歴から更新処理を再実行しません。不明な書き込みは[メタデータと元 ID の手順](control-api.md#operational-details)で確認します。

ローカル `event` は `{topic,data?}` で単位型では data がない場合もあります。リモート `event.params` は文字列 sequence、イベント ID、リソース版、actor を持つ ReliableEvent です。`event.gap` は別メソッドです。以下は形式例で、失われた副作用の永続 replay ではありません。

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

以下の SDK 統合例には remote-profiles が必要です。投影 callback はアプリが実装し acknowledge より先に完了します。[RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) に型付き baseline・cursor・再開契約があります。

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```
