---
title: "代理与类型化 Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "zh"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
translationRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
order: 80
---

# 代理与类型化 Rust SDK {#agents-and-the-typed-rust-sdk}

## 目标与前置条件 {#goal-and-prerequisites}

面向桌面使用的同一宿主编写脚本或代理。你需要运行中的隔离宿主、匹配的 `tiktools` CLI，以及[发现接口](control-api.md)访问。SDK 位于 `crates/tiktools-client`，公开直接、已连接本地及可选远程后端，不会在普通客户端路径中引入另一个业务核心。

## 从只读发现开始 {#start-with-read-only-discovery}

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

生成操作前，先确定支持的能力和参数 schema。不能仅凭 `system.info` 推断 TikTok 已连接、OBS 可用或允许修改。`system.doctor` 会区分运行时探测、跳过的检查与构建能力。

Rust 异步函数可附加到现有所有者：

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

调用者提供 Tokio 运行时。类型化方法委托给已注册的 RPC 方法，SDK 对等集成测试保障覆盖率。工作流程、自动化或所有者日志需要版本和命令回执时，使用 `call_with_meta` 或 `call_value_with_meta`；仅返回领域数据的辅助方法有意保持更简单的返回形式。

## 提交有明确意图的变更 {#submit-intentional-changes}

持久化操作需保留方法、规范化参数、原始命令 ID、宿主实例及发现要求的读取修订。提交前保存用户候选项。远程自动化草稿必须明确保持禁用且归作者所有；它等待所有者审查，并不代表获准执行。所有者采纳与激活具有独立的本地契约。

## 预期结果、状态与恢复 {#expected-result-status-and-recovery}

代理获得与其他客户端相同的验证、事件和失败结果。代理访问已实现；权限取决于选定后端和当前授权。确认结果未知时，应查询原始结果，不能自动换新 ID 重新提交。封闭式远程摘要无法重建完整定义，因此需私下保留提交的源码。

添加方法后，运行 `bun scripts/task.ts rust test -p tiktools-client --test parity --locked`。[SDK 源码](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs)及规范注册表负责定义签名；历史代理示例可能落后于新的所有者恢复协调器。

## 面向代理的无界面 Teams {#headless-teams}

编译具有主机支持的 CLI，或使用便携发行包中版本一致的 CLI。将所有者作为持久进程运行，并为其设置绝对路径、独立的 `TIKTOOLS_HOME`。另一个终端中的所有者命令必须使用同一目录。

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` 登录所有者运行时，`remote cloud login` 登录 controller 上下文；两者的账户元数据和设备密钥保持独立。登录通过真实的浏览器交接完成，不需要桌面窗口。`--non-interactive` 绝不启动登录；缺少凭据时返回 `authentication_required`。

所有者先创建团队、注册主机设备并启动服务。使用 `teams people search NICKNAME` 查找接收者 ID，再创建绑定该接收者的邀请。令牌只写入一次新建的私有文件，标准输出仅含元数据。必须明确批准申请，并在连接前配置代理的原生授权。

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

批准后使用 `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` 建立可重复使用的访问配置。`teams list --joined` 查询 controller 的团队资格；`teams list --owned` 查询所有者的团队。`teams doctor` 检查所有者，`teams doctor --context controller` 检查 controller 的认证与团队资格，`remote check agent` 验证真实连接。

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

持久客户端每行接收一条 JSON-RPC 请求并保留请求 ID。写操作要求 `confirm: true`；持久写操作还需要文档规定的命令和主机元数据。输入结束、终止信号或连接关闭都会释放客户端。请求大小和超时均有限制；超时的写入仍可能已经提交。

验收运行器使用已编译 CLI、真实的一次性后端、合成账户，以及相互独立的所有者和 controller 进程。请提供包含 CLI 验收 fixture 且已安装 pgrust 工具的 RemakeBackend checkout。开发二进制允许运行时使用独立测试源。发行二进制仍固定到编译时的源：为固定的 loopback TLS 端口编译 staging release，并传入 `--cloud-port PORT`。固定到生产源的二进制会在认证前被拒绝，不会启动远程 workflow。

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

新构建的安装包还支持 `./TikTools.AppImage --cli --build-info` 和 `./TikTools.AppImage --cli --json teams doctor`，会在初始化图形界面前转交给 CLI。以前发布的 AppImage 不会自动获得此入口。

在不同 CLI 进程间复用 controller 配置需要可用的操作系统凭据存储。
