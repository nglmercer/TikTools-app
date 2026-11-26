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
