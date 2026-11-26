| コマンド | 概要 |
| --- | --- |
| `tiktools api discover [--format json&#124;human]` | ホストレジストリからすべてのメソッドをライブで一覧表示 |
| `tiktools api schema <method> [--format json&#124;human]` | メソッド1つの説明、フラグ、スキーマ |
| `tiktools api call <method> [params-json] [--timeout SEC] [--dry-run] [--confirm] [--redact&#124;--secrets-visible] [--explain] [--format json&#124;human] [--expected-revision N] [--command-id ID --host-instance-id ID] [--include-meta]` | メソッドを検証して実行；破壊的メソッドには --confirm が必要 |
| `tiktools api events [--topics t,...] [--timeout SEC] [--max-events N]` | NDJSON ドメインイベントをストリーム |
| `tiktools api event get <name> [--type kind]... [--author text] [--limit N]` | 有界フィードジャーナルを検索 |
| `tiktools api verify [--coverage&#124;--smoke] [--sandbox] [--timeout SEC]` | ホストに対してコントロールサーフェスを検証 |
