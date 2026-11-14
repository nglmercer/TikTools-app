---
title: "永続化と確実な復旧"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7ec9d94e484af17e8e82aaaa2122cc47ecd206cf9306047795d6aaa72eea2edb"
translationRevision: "4b0d4e93e515898bc2dd78d1783299890afd6aebe472be9c8734a2fbcb5ef56d"
order: 60
---

# 永続化と確実な復旧 {#persistence-and-durable-recovery}

## 目的と前提条件 {#goal-and-prerequisites}

担当サービスを通して永続状態を変え、復旧可能な結果を保ちます。使い捨てプロファイルを使い、DB コピー前にホストを閉じ、バージョン付きリソース編集前に[制御メタデータ](control-api.md) を読みます。`TIKTOOLS_HOME` がルートを選び、`AppPaths` がデータと非公開ストアを解決します。

## 保存先を特定する {#identify-the-store}

`crates/tiktools-core/src/paths.rs` が `data/tiktok-points.db`、`data/tiktok-automation.db`、`data/tiktok-analytics.db` を定義します。各 SQLite サービスがスキーマを所有します。ポイント・視聴者と自動化定義は別台帳、統計も独立 DB で、古い2 DB 構成図には統計がありません。

起動時、保存先がない場合だけチェックアウト内の旧 DB を環境のアプリデータへコピーできます。既存の保存先は上書きしません。ネイティブ実行の変更中も JSON 定義を読み取れます。ユーザープロファイルの削除や Vue からのテーブル再書き込みで移行を実装しないでください。

## 復旧可能な変更を行う {#make-a-recoverable-change}

1. サービスのトランザクションとスキーマ・移行テストを探します。
2. 対応ドメインでは業務変更、リビジョン、必要なジャーナル・outbox 予約を同じトランザクションに保存します。
3. 削除時のリビジョン墓標を保持します。再利用 ID は削除前の編集リビジョンを受け付けてはいけません。
4. 既存 outbox からコミット済み無効化情報を公開します。配信再試行で元の処理を繰り返しません。
5. 中断後は同じ非公開ホームを開き、別変更の前に元のコマンドを解決します。

起動済みテストホストでの読み取り専用確認例です。

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## 期待される結果と提供状況 {#expected-result-and-status}

対応する永続ワークフロー、ポイント、所有者ジャーナルは再起動後も元の結果を保持します。受領記録は過去のコマンドを表し、現在の状態は別に読みます。Cloud ワークスペースの保存にはオリジン・アカウント分割と世代検査がありますが、ローカル SQLite の権威を置き換えません。外部処理やドメインすべてが同じ復旧範囲を持つわけではありません。

## 問題が起きたとき {#recovery}

オフラインバックアップでは SQLite と存在する WAL/SHM をまとめて保管します。UI タイムアウトはロールバックの証拠ではありません。`revision_conflict` は下書きを保持して新しい読み取りと比較します。外部処理が不確実ならフィードを再実行せず保持した処理記録を調べます。[パスの正本](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs) と [Cloud 復旧](cloud.md) で別々の境界を確認します。

## 運用の詳細 {#operational-details}

試験・バックアップ前に `tiktools api call system.info --json` で実パスを確認します。`TIKTOOLS_HOME` がルートを選び、`TIKTOOLS_DATA_DIR`、`TIKTOOLS_PLUGINS_DIR`、`TIKTOOLS_PLUGIN_DATA_DIR`、`TIKTOOLS_PLUGIN_STORE_DIR`、`TIKTOOLS_LOG_DIR`、`TIKTOOLS_TEMP_DIR` が派生先を個別に上書きします。相対パスは起動ディレクトリ基準です。home の分離だけでは継承変数を無効化できないため先に確認・解除します。

Analytics は集計データを専用ライターで書きます。現在のキューは 1,024 件、バッチ 128 件、flush 期限 10 秒です。データベースパスから隣接 `*.pending.json` を作ります。起動時に ID 付きバッチを再適用し、回復ファイルを除いてから `ready` が新規処理を許可します。読めない・16 MiB 超の回復ファイル、使用不能な DB、回復ファイル削除失敗では受け付けを停止します。DB と回復ファイルを保持し、`ready`・未処理件数・エラーを確認して修復前に保存します。健全に見せるための削除はしません。ポイントログは別の耐久性契約です。

`system.info` は実 home/dataDir を返します。他のディレクトリは別端末ではなくホストの実起動環境で解決します。`system.health` は集計 ready/未処理と LIVE 入口カウンターを提供します。
