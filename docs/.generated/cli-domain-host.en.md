| Command | Summary |
| --- | --- |
| `tiktools host --stdio [--events]` | headless host on stdio; streams events with --events |
| `tiktools host --ipc` | headless host on local IPC; refuses when a host is running |
| `tiktools rpc <method> [params-json] &#124; rpc --stdio [--events]` | untyped passthrough to any host method |
| `tiktools rpc --stdio [--events]` | NDJSON request/response loop on stdio |
