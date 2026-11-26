`tiktools [--json] [--standalone] <command> [args]`

### `tiktools plugin` — プラグインのインストールと管理

| コマンド | 概要 |
| --- | --- |
| `tiktools plugin list` | インストール済みプラグインを一覧表示 |
| `tiktools plugin get <id>` | プラグインを1つ読み取る |
| `tiktools plugin install <file.plugin> [--replace] &#124; plugin install <id[@version]&#124;url> [--yes] [--no-wait] [--allow-unsigned]` | ファイル、id@version または url からインストール |
| `tiktools plugin uninstall <id> [--purge]` | プラグインをアンインストール **破壊的** |
| `tiktools plugin search <query> [--category c] [--page N]` | レジストリでパッケージを検索 |
| `tiktools plugin inspect <spec>` | インストール前にパッケージ仕様を検査 |
| `tiktools plugin outdated` | 更新可能なインストール済みプラグインを一覧表示 |
| `tiktools plugin update <id> [--version v] [--yes] [--no-wait]` | プラグインを1つ更新 |
| `tiktools plugin update --all [--yes] [--no-wait]` | すべての古いプラグインを更新 |
| `tiktools plugin rollback <id> [version] [--no-wait]` | 以前のバージョンにロールバック **破壊的** |
| `tiktools plugin versions <id>` | プラグインの利用可能なバージョンを一覧表示 |
| `tiktools plugin pin <id>` | プラグインを現在のバージョンにピン留め |
| `tiktools plugin unpin <id>` | バージョンのピン留めを解除 |
| `tiktools plugin policy <id> [--channel stable&#124;beta] [--auto off&#124;notify&#124;patch&#124;minor]` | チャンネルと自動更新ポリシーを設定 |
| `tiktools plugin jobs` | パッケージジョブを一覧表示 |
| `tiktools plugin job <job-id>` | パッケージジョブを1つ検査 |
| `tiktools plugin cancel <job-id>` | 実行中のパッケージジョブをキャンセル **破壊的** |
| `tiktools plugin enable <id>` | プラグインを有効化 |
| `tiktools plugin disable <id>` | プラグインを無効化 |
| `tiktools plugin start <id>` | プラグインを開始 |
| `tiktools plugin stop <id>` | プラグインを停止 **破壊的** |
| `tiktools plugin settings get <id>` | プラグイン設定を読み取る |
| `tiktools plugin settings set <id> k=v...` | プラグイン設定を書き込む |
| `tiktools plugin settings reset <id>` | プラグイン設定をリセット **破壊的** |
| `tiktools plugin health <id>` | プラグインのヘルスチェック |
| `tiktools plugin options <action-type/field>` | アクションタイプまたはフィールドのオプション |
| `tiktools plugin action <type> [--config json] [--live]` | プラグインアクションを実行 |
| `tiktools plugin commands` | インストール済みプラグインが提供するコマンドを一覧表示 |
| `tiktools plugin help [id]` | 提供されたコマンドを1つ説明 |
| `tiktools plugin <id> <command> [--params json] [--live]` | プラグイン提供コマンドを呼び出す |

### `tiktools cloud` — クラウドアカウントとポータブルバックアップ/復元（TIKTOOLS_CLOUD_ORIGIN で上書き）

| コマンド | 概要 |
| --- | --- |
| `tiktools cloud status` | クラウドアカウントの状態 |
| `tiktools cloud login` | システムブラウザでサインイン |
| `tiktools cloud logout` | サインアウト **破壊的** |
| `tiktools cloud restore` | クラウドセッションを復元 |
| `tiktools cloud backup create [--out FILE]` | ポータブルバックアップアーカイブを書き出す |
| `tiktools cloud backup preview <file>` | バックアップアーカイブを検査 |
| `tiktools cloud backup restore <file> [--yes]` | バックアップアーカイブを復元 **破壊的** |

### `tiktools publisher` — クラウドパブリッシャーフロー（サインインが必要）

| コマンド | 概要 |
| --- | --- |
| `tiktools publisher list` | パブリッシャー名前空間を一覧表示 |
| `tiktools publisher create <namespace>` | パブリッシャー名前空間を作成 |
| `tiktools publisher delete <namespace-or-publisher-id>` | 名前空間またはパブリッシャーを削除 **破壊的** |
| `tiktools publisher keys <publisher-id>` | パブリッシャーの署名キーを一覧表示 |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | パブリッシャーキーを登録；バックエンドのパスワードを stdin から読み取る |
| `tiktools publisher upload <publisher-id> <path>` | プラグインパッケージをアップロード |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | アップロードをレビューに提出 |
| `tiktools publisher release <release-id>` | リリースを作成 |
| `tiktools publisher publish <release-id>` | リリースを公開 |

### `tiktools registry` — パッケージレジストリとクラウドカタログ

| コマンド | 概要 |
| --- | --- |
| `tiktools registry list` | 設定済みレジストリを一覧表示 |
| `tiktools registry refresh` | レジストリメタデータを更新 |
| `tiktools registry package <id>` | レジストリパッケージを1つ表示 |
| `tiktools registry cloud-catalog [--query q] [--after cursor] [--generation value] [--target name]` | クラウドカタログを検索 |
| `tiktools registry cloud-summaries [--query q] [--cursor value] [--generation value] [--target name]` | クラウドカタログのサマリーページ |
| `tiktools registry cloud-versions <id> [--target name] [--generation value] [--cursor value]` | クラウドカタログパッケージのバージョン |
| `tiktools registry cloud-pagination` | クラウドカタログのページネーション機能 |
| `tiktools registry defaults` | デフォルトのレジストリ設定 |

### `tiktools processor` — イベントプロセッサー

| コマンド | 概要 |
| --- | --- |
| `tiktools processor list` | プロセッサーを一覧表示 |
| `tiktools processor status` | プロセッサーパイプラインの状態 |
| `tiktools processor test <plugin-id> <processor-id> [event-json]` | サンプルイベントに対してプロセッサーを1つ実行 |

### `tiktools live` — TikTok LIVE 接続

| コマンド | 概要 |
| --- | --- |
| `tiktools live connect --unique-id <id> --session-cookie <cookie> [--room-id <id>]` | ライブルームに接続 |
| `tiktools live pick --session-cookie <cookie>` | ライブルームを対話的に選択 |
| `tiktools live channels [--keyword text] [--offset N] [--limit N] [--session-cookie <cookie>]` | ライブチャンネルを発見；セッションなしで動作 |
| `tiktools live disconnect` | 現在のルームから切断 **破壊的** |
| `tiktools live status` | 現在の接続状態 |

### `tiktools points` — 視聴者ロイヤルティポイント

| コマンド | 概要 |
| --- | --- |
| `tiktools points config get` | ポイント設定を読み取る |
| `tiktools points config set k=v...` | ポイント設定を更新 |
| `tiktools points viewer get <id>` | 視聴者1人の残高を読み取る |
| `tiktools points adjust <id> <delta>` | 視聴者のポイントを加算または減算 |
| `tiktools points leaderboard [--limit N]` | 残高上位の視聴者 |
| `tiktools points reset [id]` | 視聴者1人またはすべての残高をリセット **破壊的** |

### `tiktools ranking` — 視聴者ランキングの定義と状態

| コマンド | 概要 |
| --- | --- |
| `tiktools ranking list` | ランキング定義を一覧表示 |
| `tiktools ranking metrics` | ランキング定義で利用可能なメトリクス |
| `tiktools ranking create --record <json&#124;@file>` | ランキング定義を作成 |
| `tiktools ranking update --record <json&#124;@file>` | ランキング定義を更新 |
| `tiktools ranking get <id>` | ランキングを1つ読み取る |
| `tiktools ranking snapshot <id>` | ランキングの現在の順位 |
| `tiktools ranking delete <id>` | ランキングを削除 **破壊的** |
| `tiktools ranking reset <id>` | 蓄積状態をリセット **破壊的** |
| `tiktools ranking pause <id>` | 蓄積を一時停止 |
| `tiktools ranking resume <id>` | 蓄積を再開 |
| `tiktools ranking contribute --record <json&#124;@file>` | 外部の貢献を追加 |

### `tiktools competition` — 時間制限付き視聴者コンペティション

| コマンド | 概要 |
| --- | --- |
| `tiktools competition list` | コンペティションを一覧表示 |
| `tiktools competition get <id>` | コンペティションを1つ読み取る |
| `tiktools competition create --record <json&#124;@file>` | コンペティションを作成 |
| `tiktools competition update --record <json&#124;@file>` | コンペティションを更新 |
| `tiktools competition delete <id>` | コンペティションを削除 **破壊的** |
| `tiktools competition state <id>` | 現在のコンペティション状態 |
| `tiktools competition results <id>` | 最終または中間結果 |
| `tiktools competition action --record <json&#124;@file>` | コンペティションアクションを記録 |
| `tiktools competition mark-action-uncertain --record <json&#124;@file>` | アクションを不確実としてフラグ |
| `tiktools competition resolve-action --record <json&#124;@file>` | 不確実なアクションを解決 |
| `tiktools competition start <id>` | コンペティションを開始 |
| `tiktools competition finish <id>` | コンペティションを終了 **破壊的** |
| `tiktools competition candidates <id>` | 勝者候補 |
| `tiktools competition select-winner --record <json&#124;@file>` | 勝者を選択 |
| `tiktools competition pause <id>` | コンペティションを一時停止 |
| `tiktools competition resume <id>` | コンペティションを再開 |
| `tiktools competition cancel <id>` | コンペティションをキャンセル **破壊的** |

### `tiktools automation` — イベントとアクションの自動化ルール

| コマンド | 概要 |
| --- | --- |
| `tiktools automation list [--kind event&#124;action&#124;all]` | 自動化ルールを一覧表示 |
| `tiktools automation context [--event-type t]` | ルールで利用可能なテンプレートコンテキスト |
| `tiktools automation get <id> [--kind event&#124;action]` | ルールを1つ読み取る |
| `tiktools automation create --record json [--kind event&#124;action]` | ルールを作成 |
| `tiktools automation update <id> --record json [--kind event&#124;action]` | ルールを更新 |
| `tiktools automation delete <id> [--kind event&#124;action]` | ルールを削除 **破壊的** |
| `tiktools automation enable <id> [--kind event&#124;action]` | ルールを有効化 |
| `tiktools automation disable <id> [--kind event&#124;action]` | ルールを無効化 |
| `tiktools automation test (--id <id> &#124; --record json) [--kind event&#124;action] [--trigger t] [--event json]` | サンプルイベントに対してルールをドライラン |
| `tiktools automation fire --trigger <type> [--event json] [--record json]` | トリガーを手動で発火 |
| `tiktools automation moderation-penalty --points <delta>` | ポイントでモデレーションペナルティを適用 |

### `tiktools globals` — プラグイン間で共有されるキー/バリューグローバル

| コマンド | 概要 |
| --- | --- |
| `tiktools globals list` | すべてのグローバルキーを一覧表示 |
| `tiktools globals get <key>` | グローバルを1つ読み取る |
| `tiktools globals set <key> <value>` | グローバルを1つ書き込む |
| `tiktools globals delete <key>` | グローバルを1つ削除 **破壊的** |

### `tiktools workflow` — ビジュアルワークフローグラフ

| コマンド | 概要 |
| --- | --- |
| `tiktools workflow list` | 保存されたワークフローグラフを一覧表示 |
| `tiktools workflow get <id>` | グラフを1つ読み取る |
| `tiktools workflow save --record json` | グラフを作成または置換 |
| `tiktools workflow delete <id>` | グラフを削除 **破壊的** |
| `tiktools workflow enable <id>` | グラフを有効化 |
| `tiktools workflow disable <id>` | グラフを無効化 |

### `tiktools media` — メディアファイルの検証と再生

| コマンド | 概要 |
| --- | --- |
| `tiktools media validate <path> [--kind audio&#124;video&#124;image&#124;other]` | アクションで使用する前にメディアファイルを検証 |

### `tiktools system` — ホスト情報と診断

| コマンド | 概要 |
| --- | --- |
| `tiktools system info` | ホストバージョンとランタイム情報 |
| `tiktools system health` | コンポーネントヘルスレポート |
| `tiktools system snapshot` | デバッグ用の完全な状態スナップショット |
| `tiktools system doctor` | 一般的なセットアップ問題を診断 |
| `tiktools system request-input-access` | デスクトップユーザーに入力アクセスを要求 |
| `tiktools system shutdown` | 実行中のホストをシャットダウン **破壊的** |

### `tiktools template` — 再利用可能な自動化テンプレートとプロファイル

| コマンド | 概要 |
| --- | --- |
| `tiktools template import <file> [--param k=v]... [--event-name n] [--action-name n] [--dry-run] [--no-save] [--disabled]` | テンプレートを自動化ルールとしてインポート |
| `tiktools template export --event <id> --out <file> [--title t] [--id template-id]` | イベントルールをテンプレートとしてエクスポート |
| `tiktools template list` | ギャラリーのテンプレートを一覧表示 |
| `tiktools template delete <template-id>` | テンプレートを削除 **破壊的** |
| `tiktools template profile-import <file> [--param k=v]... [--dry-run] [--no-save] [--disabled]` | 自動化プロファイル全体をインポート |
| `tiktools template profile-export --out <file> [--name n]` | 自動化プロファイルをエクスポート |

### `tiktools api` — ホストメソッド上のエージェント指向の汎用コントロールサーフェス

| コマンド | 概要 |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | ホストレジストリからすべてのメソッドをライブで一覧表示 |
| `tiktools api schema <method> [--format json&#124;human]` | メソッド1つの説明、フラグ、スキーマ |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | メソッドを検証して実行；破壊的メソッドには --confirm が必要 |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | NDJSON ドメインイベントをストリーム |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | 有界フィードジャーナルを検索 |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | ホストに対してコントロールサーフェスを検証 |

### `tiktools host` — ヘッドレスホストランタイムと生の rpc パススルー

| コマンド | 概要 |
| --- | --- |
| `tiktools host --stdio [--events]` | stdio 上のヘッドレスホスト；--events でイベントをストリーム |
| `tiktools host --ipc` | ローカル IPC 上のヘッドレスホスト；ホスト実行中は拒否 |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | 任意のホストメソッドへの型なしパススルー |
| `tiktools rpc --stdio [--events]` | stdio 上の NDJSON リクエスト/レスポンスループ |

### `tiktools web` — ローカルブラウザアダプター（local-web フィーチャーが必要）

| コマンド | 概要 |
| --- | --- |
| `tiktools web --assets <directory> [--port N] [--join-origin URL] [--multi-session]` | ローカルブラウザアダプターを提供；host --ipc が実行中である必要がある |
| `tiktools web --print-code [--port N]` | 再起動せずにアクティブなペアリングコードを印刷 |
| `tiktools web --reset-session [--port N]` | すべてのペアリング済みブラウザを取り消し、新しいコードを印刷 **破壊的** |

### 規約

- **ホスト解決**: コマンドはローカル IPC 経由で実行中のホストに対して実行されます。ホストが実行されていない場合、2 つ目のランタイムを起動する代わりに host_unavailable で失敗します。軽量プローブとして `tiktools --json system info` を使用してください：終了コード 0 はホストが応答したことを、3 は IPC ホストが利用不可であることを意味します。--standalone はコマンドを分離されたプロセス内ランタイムに対して実行します。
- **出力**: stdout は結果を運び、診断は stderr に送られます。--json は生の結果オブジェクトを出力します。指定がない場合、リストは項目ごとに 1 行で出力します。json 値は @path でファイルから読み取れます。人間向けヘルプは表示ロケールに従います：TIKTOOLS_LANG は LC_ALL/LC_MESSAGES/LANG チェーンを上書きし、マシン出力はロケールに依存しません。
- **終了コード**: 0 成功、1 操作エラー、2 使用法エラー、3 転送失敗。
- **シークレット**: live connect/pick はセッション cookie を --session-cookie または環境変数 TIKTOOLS_SESSION_COOKIE から読み取ります（フラグが優先）。値が診断に表示されることはありません。api call は --secrets-visible が渡されない限り出力からシークレットを伏せ、--confirm なしでは破壊的メソッドを拒否します。
