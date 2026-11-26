| Command | Summary |
| --- | --- |
| `tiktools publisher list` | list publisher namespaces |
| `tiktools publisher create <namespace>` | create a publisher namespace |
| `tiktools publisher delete <namespace-or-publisher-id>` | delete a namespace or publisher **destructive** |
| `tiktools publisher keys <publisher-id>` | list signing keys of a publisher |
| `tiktools publisher enroll <publisher-id> [--key-id <id>]` | enroll a publisher key; reads the backend password from stdin |
| `tiktools publisher upload <publisher-id> <path>` | upload a plugin package |
| `tiktools publisher submit <upload-id> <publisher-key-id> <plugin-id> <version> [--target any]` | submit an upload for review |
| `tiktools publisher release <release-id>` | create a release |
| `tiktools publisher publish <release-id>` | publish a release |
