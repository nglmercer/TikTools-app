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
