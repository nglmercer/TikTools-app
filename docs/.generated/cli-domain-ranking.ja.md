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
