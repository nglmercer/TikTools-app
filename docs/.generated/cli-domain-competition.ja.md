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
