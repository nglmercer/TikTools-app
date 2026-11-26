| コマンド | 概要 |
| --- | --- |
| `tiktools host --stdio [--events]` | stdio 上のヘッドレスホスト；--events でイベントをストリーム |
| `tiktools host --ipc` | ローカル IPC 上のヘッドレスホスト；ホスト実行中は拒否 |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | 任意のホストメソッドへの型なしパススルー |
| `tiktools rpc --stdio [--events]` | stdio 上の NDJSON リクエスト/レスポンスループ |
