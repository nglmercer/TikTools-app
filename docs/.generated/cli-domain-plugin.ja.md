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
