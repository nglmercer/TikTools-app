---
title: "エージェントと型付き Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "81526e0de1483b13f1df159576cb69f37ce2c5e3cf072ee9db199cd6c8c84dfb"
translationRevision: "17368ab881386205f22473fa83fd7adc31d085f3dc9f2e8a8b593eafc151318f"
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

## エージェント向けのヘッドレス Teams {#headless-teams}

ホスト機能を有効にして CLI をビルドするか、ポータブルリリースに含まれる同じバージョンの CLI を使用します。所有者を永続プロセスとして実行し、絶対パスの独立した `TIKTOOLS_HOME` を指定します。別の端末からの所有者コマンドも同じディレクトリを使います。

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` は所有者ランタイム、`remote cloud login` は controller コンテキストを認証します。アカウントのメタデータとデバイス鍵は分離されたままです。どちらも実際のブラウザによる引き継ぎを使い、デスクトップウィンドウは不要です。`--non-interactive` はログインを開始せず、資格情報がなければ `authentication_required` を返します。

所有者はチームを作成し、ホストデバイスを登録してサービスを開始します。`teams people search NICKNAME` で宛先 ID を調べ、その宛先に紐づく招待を作成します。トークンは新規の非公開ファイルに一度だけ保存され、標準出力にはメタデータのみが含まれます。申請を明示的に承認し、接続前にエージェントのネイティブ権限を設定してください。

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

承認後に `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` を使うと再利用可能な接続を作成できます。`teams list --joined` は controller の参加チーム、`teams list --owned` は所有チームを取得します。`teams doctor` は所有者、`teams doctor --context controller` は controller の認証と所属を調べ、`remote check agent` は実際の接続を検証します。

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

永続クライアントは一行に一つの JSON-RPC リクエストを受け取り、ID を保持します。書き込みには `confirm: true` が必要です。永続的な書き込みには文書化されたコマンドとホストのメタデータも必要です。EOF、終了シグナル、接続終了でクライアントを解放します。サイズと時間には上限があり、タイムアウトした書き込みでも確定している場合があります。

受け入れランナーはコンパイル済み CLI、実際の使い捨てバックエンド、合成アカウント、別々の所有者と controller プロセスを使います。CLI 受け入れ fixture とインストール済み pgrust ツールを含む RemakeBackend checkout を指定します。開発バイナリは実行時にテスト用 origin を使えます。リリースはビルド時の origin を維持するため、固定 loopback TLS ポート向けに staging release をビルドして `--cloud-port PORT` を指定します。本番 origin に固定されたバイナリは認証前に拒否され、リモート workflow は開始しません。

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

新しいパッケージは `./TikTools.AppImage --cli --build-info` と `./TikTools.AppImage --cli --json teams doctor` にも対応し、GUI 初期化前に CLI へ処理を渡します。公開済みの古い AppImage にこの入口が自動追加されることはありません。

CLI プロセス間で controller プロファイルを再利用するには、OS の資格情報ストアが利用可能である必要があります。

## 運用の詳細 {#operational-details}

副作用前に schema を取得し、`--dry-run --explain` で CAS・加算・永続性・破壊性を確認します。[分離した読み取り/CAS/元結果の手順](control-api.md#operational-details)と[スナップショット/適用/acknowledge 回復](remote-client.md#operational-details)を使います。ツールはメタデータ・元 payload/コマンド ID・呼出範囲を保持します。timeout 後の ID 変更は副作用を重複させます。`--confirm` は破壊操作の確認であり権限拡大ではありません。
