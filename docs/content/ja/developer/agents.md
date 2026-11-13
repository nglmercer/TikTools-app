---
title: "エージェントと型付き Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
translationRevision: "7c0b540c4f1b4e1953b3007092ce5440109f355d8ad06eb763aa60d3616f23d0"
order: 80
---

# エージェントと型付き Rust SDK {#agents-and-the-typed-rust-sdk}

## 目的と前提条件 {#goal-and-prerequisites}

デスクトップと同じホストを使うスクリプトやエージェントを作ります。分離したホスト、対応する `tiktools` CLI、[機能検出](control-api.md) へのアクセスを用意します。SDK は `crates/tiktools-client` で、直接、接続済みローカル、任意のリモートバックエンドを公開します。通常のクライアント経路に別の業務コアを作りません。

## 読み取り専用の機能検出から始める {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

アクションを生成する前に対応機能とパラメータースキーマを解決します。`system.info` だけで TikTok 接続、OBS の利用可否、変更権限を推測しないでください。`system.doctor` は実行時検査・省略された検査とビルド機能を区別します。

Rust の非同期関数から既存の所有ホストへ接続できます。

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

呼び出し側が Tokio ランタイムを用意します。型付きメソッドは登録済み RPC に委譲し、SDK の整合性統合テストが網羅性を守ります。ワークフロー、自動化、所有者ジャーナルにバージョンとコマンド受領記録が必要なら `call_with_meta` または `call_value_with_meta` を使います。ドメインだけを返すヘルパーは意図的に単純な戻り値を保ちます。

## 意図した変更を送信する {#submit-intentional-changes}

永続的な操作では、メソッド、正規化パラメーター、元のコマンド ID、ホストインスタンス、機能検出が要求する読み取りリビジョンを保持します。送信前にユーザーの候補を保存します。リモートの自動化下書きは明示的に無効で、作成者の所有のままにします。所有者確認待ちであり、実行許可ではありません。所有者による採用と有効化には別々のローカル契約があります。

## 期待される結果・提供状況と対処 {#expected-result-status-and-recovery}

エージェントも他のクライアントと同じ検証、イベント、失敗を受け取ります。アクセスは実装済みで、権限は選択バックエンドと現在の付与に依存します。応答確認が不明なら元の結果を照会し、新しい ID で自動再送しません。限定されたリモート要約から完全な定義は復元できないので、送信ソースを非公開で保持します。

メソッド追加後は `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` を実行します。シグネチャの正本は [SDK ソース](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) と正規レジストリです。過去のエージェント例は新しい所有者復旧コーディネーターに追従していない場合があります。
