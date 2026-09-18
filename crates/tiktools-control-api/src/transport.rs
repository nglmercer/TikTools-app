//! NDJSON transports: stdio and persistent local IPC.
//!
//! All transports share one framing rule: one JSON request per line, one
//! JSON response per line. Event notifications interleave on the same
//! stream when event streaming is enabled.

use std::sync::Arc;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::{event_notification, ApiError, ControlApi, RpcResponse};

/// Hard cap for one request line (bytes, including the newline).
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// Hard cap for the serialized `params` payload of one request.
pub const MAX_PARAMS_BYTES: usize = 256 * 1024;
/// Local IPC endpoint name (`tiktools-control.sock` / pipe).
pub const IPC_NAME: &str = "tiktools-control";
/// Listener shutdown poll interval.
const SHUTDOWN_POLL: std::time::Duration = std::time::Duration::from_millis(250);

/// Serves JSON-RPC over stdin/stdout until EOF or `system.shutdown`.
/// `stream_events` interleaves domain-event notifications on stdout.
pub async fn run_stdio(api: ControlApi, stream_events: bool) -> std::io::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    serve_stream(
        &api,
        tokio::io::BufReader::new(stdin),
        stdout,
        stream_events,
    )
    .await
}

/// Serves JSON-RPC over persistent local IPC with event streaming:
/// a Unix domain socket on Unix, a named pipe on Windows.
pub async fn run_ipc(api: ControlApi) -> std::io::Result<()> {
    run_ipc_shared(Arc::new(api)).await
}

/// Serves local IPC from a shared [`ControlApi`]. The desktop host uses this
/// so its WebView, control router, and IPC server all share one `AppCore`.
/// The loop exits once the core starts shutting down.
///
/// The server holds the OS control-host ownership primitive for its whole
/// lifetime and fails with `AddrInUse` when another host already owns the
/// production endpoint.
pub async fn run_ipc_shared(api: Arc<ControlApi>) -> std::io::Result<()> {
    run_ipc_shared_with_ready(api, || {}).await
}

/// Serves local IPC like [`run_ipc_shared`], invoking `on_ready` exactly
/// once after the endpoint is actually listening (Unix bind / first pipe
/// instance). Ownership or bind failures return before it ever fires, so
/// retry loops can clear degraded health only on genuine recovery.
pub async fn run_ipc_shared_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    let _ownership = crate::ownership::acquire_control_host_ownership()?;
    #[cfg(unix)]
    {
        run_ipc_unix_with_ready(api, on_ready).await
    }
    #[cfg(windows)]
    {
        run_ipc_windows_with_ready(api, on_ready).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = api;
        let _ = on_ready;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "local IPC is only available on Unix and Windows",
        ))
    }
}

#[cfg(unix)]
pub(crate) fn ipc_socket_path() -> std::path::PathBuf {
    tiktools_core::paths::AppPaths::from_environment()
        .root
        .join(format!("{IPC_NAME}.sock"))
}

#[cfg(unix)]
async fn run_ipc_unix_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::UnixListener;

    let path = ipc_socket_path();
    if path.exists() {
        // The ownership lock is already held here (acquired before this
        // function runs), so a live owner cannot exist: a connect probe
        // distinguishes its stale socket (unlink) from a foreign bind,
        // which still fails below with `AddrInUse`.
        if tokio::net::UnixStream::connect(&path).await.is_err() {
            let _ = std::fs::remove_file(&path);
        }
    }
    let listener = UnixListener::bind(&path)?;
    // Per-user endpoint: only the owner may connect to the control socket.
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!(path = %path.display(), "control IPC listening");
    on_ready();
    loop {
        if api.core().is_shutdown() {
            let _ = std::fs::remove_file(&path);
            return Ok(());
        }
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = result?;
                let api = Arc::clone(&api);
                tokio::spawn(async move {
                    let (read, write) = stream.into_split();
                    if let Err(error) =
                        serve_stream(&api, tokio::io::BufReader::new(read), write, true).await
                    {
                        tracing::debug!(%error, "control IPC connection ended");
                    }
                });
            }
            // Wake periodically so `system.shutdown` stops the listener
            // even while no client is connecting.
            _ = tokio::time::sleep(SHUTDOWN_POLL) => {}
        }
    }
}

#[cfg(windows)]
pub(crate) fn ipc_pipe_name() -> std::io::Result<String> {
    // Test override so integration tests isolate their endpoint instead
    // of touching the well-known production pipe. Unix isolates through
    // TIKTOOLS_HOME already, so no override is needed there.
    if let Ok(name) = std::env::var("TIKTOOLS_IPC_NAME") {
        if !name.is_empty()
            && name.len() <= 64
            && name.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '-' || character == '_'
            })
        {
            return Ok(format!(r"\\.\pipe\{name}"));
        }
    }
    // Per-user production pipe: the pipe namespace is machine-wide while
    // the ownership mutex is session-scoped, so a static name would let
    // two users/sessions collide. The SID suffix isolates each user; the
    // ACL below enforces it. There is deliberately no fallback to the
    // legacy machine-wide name.
    let sid = pipe_security::current_user_sid_string()?;
    Ok(format!(r"\\.\pipe\{IPC_NAME}-{sid}"))
}

#[cfg(windows)]
async fn run_ipc_windows_with_ready<F>(api: Arc<ControlApi>, on_ready: F) -> std::io::Result<()>
where
    F: FnOnce() + Send,
{
    use tokio::net::windows::named_pipe::ServerOptions;

    let name = ipc_pipe_name()?;
    let mut on_ready = Some(on_ready);
    let mut first = true;
    loop {
        if api.core().is_shutdown() {
            return Ok(());
        }
        // The first instance claims the name (proving no live owner) and
        // locks the ACL down; later turns add instances to our own name.
        let server = if first {
            first = false;
            let claimed = pipe_security::claim_pipe_instance(&name).await?;
            tracing::info!(pipe = %name, "control IPC listening");
            claimed
        } else {
            ServerOptions::new()
                .first_pipe_instance(false)
                .create(&name)?
        };
        // Ready exactly once, after the first pipe instance exists: each
        // loop turn creates a fresh instance, so only the first counts.
        if let Some(ready) = on_ready.take() {
            ready();
        }
        tokio::select! {
            result = server.connect() => {
                result?;
                let api = Arc::clone(&api);
                tokio::spawn(async move {
                    let (read, write) = tokio::io::split(server);
                    if let Err(error) =
                        serve_stream(&api, tokio::io::BufReader::new(read), write, true).await
                    {
                        tracing::debug!(%error, "control IPC connection ended");
                    }
                });
            }
            // Wake periodically so `system.shutdown` stops the listener
            // even while no client is connecting.
            _ = tokio::time::sleep(SHUTDOWN_POLL) => {}
        }
    }
}

/// Windows pipe ownership and isolation. The pipe namespace is
/// machine-wide, so ownership is enforced here at bind time, layered over
/// the session-scoped ownership mutex (which stays as a fast same-session
/// guard and test hook):
///
/// * the production pipe name carries the current user SID, so two users
///   never share a pipe;
/// * the first instance is created with `first_pipe_instance`, so a second
///   server for the same name fails instead of splitting clients across
///   two runtimes;
/// * the pipe DACL grants access to the current user only.
///
/// A name that already exists with no live listener (crashed server whose
/// name is kept alive by stale client handles) is taken over after a
/// bounded live probe, so crash recovery is never bricked.
#[cfg(windows)]
mod pipe_security {
    use std::io;

    use tokio::net::windows::named_pipe::NamedPipeServer;
    use windows_sys::Win32::Foundation::{
        CloseHandle, LocalFree, ERROR_SUCCESS, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Security::Authorization::{
        SetEntriesInAclW, EXPLICIT_ACCESS_W, GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, TRUSTEE_IS_SID,
        TRUSTEE_IS_USER, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        GetSidIdentifierAuthority, GetSidSubAuthority, GetSidSubAuthorityCount,
        GetTokenInformation, InitializeSecurityDescriptor, SetSecurityDescriptorDacl, TokenUser,
        NO_INHERITANCE, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    struct TokenHandle(*mut core::ffi::c_void);

    impl Drop for TokenHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn open_current_process_token() -> io::Result<TokenHandle> {
        let mut token: *mut core::ffi::c_void = std::ptr::null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(TokenHandle(token))
    }

    /// Reads the token user SID into `info` (which must outlive the
    /// returned pointer).
    fn token_user_sid(
        token: &TokenHandle,
        info: &mut [u8; 256],
    ) -> io::Result<windows_sys::Win32::Security::PSID> {
        unsafe {
            let mut returned = 0u32;
            if GetTokenInformation(
                token.0,
                TokenUser,
                info.as_mut_ptr().cast(),
                info.len() as u32,
                &mut returned,
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            let user = info.as_ptr().cast::<TOKEN_USER>().read_unaligned();
            let sid = user.User.Sid;
            if sid.is_null() {
                return Err(io::Error::other("process token has no user SID"));
            }
            Ok(sid)
        }
    }

    /// `S-1-5-21-...-1001` form of the current user SID, from the process
    /// token. Only ASCII alphanumerics and dashes: safe in pipe names.
    pub fn current_user_sid_string() -> io::Result<String> {
        let token = open_current_process_token()?;
        let mut info = [0u8; 256];
        let sid = token_user_sid(&token, &mut info)?;
        unsafe {
            let authority = GetSidIdentifierAuthority(sid).read();
            let mut value: u64 = 0;
            for byte in authority.Value {
                value = (value << 8) | u64::from(byte);
            }
            let mut sid_text = format!("S-1-{value}");
            let count = GetSidSubAuthorityCount(sid).read();
            for index in 0..count {
                let sub = GetSidSubAuthority(sid, u32::from(index)).read();
                sid_text.push_str(&format!("-{sub}"));
            }
            Ok(sid_text)
        }
    }

    struct AclGuard(*mut core::ffi::c_void);

    impl Drop for AclGuard {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }

    /// Owned inputs for one creation attempt. Built synchronously and
    /// dropped before any `.await`: the raw pointers it holds must never
    /// cross an await point (the claim future is spawned on the runtime).
    struct ClaimSetup {
        _token: TokenHandle,
        _info: Box<[u8; 256]>,
        _acl: AclGuard,
        descriptor: Box<SECURITY_DESCRIPTOR>,
    }

    impl ClaimSetup {
        fn build() -> io::Result<Self> {
            let token = open_current_process_token()?;
            let mut info = Box::new([0u8; 256]);
            let sid = token_user_sid(&token, &mut info)?;
            let explicit = EXPLICIT_ACCESS_W {
                grfAccessPermissions: GENERIC_READ | GENERIC_WRITE,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: NO_INHERITANCE,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_USER,
                    ptstrName: sid.cast(),
                },
            };
            // The pipe keeps its own copy of the descriptor; the heap ACL
            // is freed once creation returns.
            let mut acl: *mut core::ffi::c_void = std::ptr::null_mut();
            let status = unsafe {
                SetEntriesInAclW(
                    1,
                    &explicit,
                    std::ptr::null(),
                    &mut acl as *mut _ as *mut *mut _,
                )
            };
            if status != ERROR_SUCCESS {
                return Err(io::Error::from_raw_os_error(status as i32));
            }
            let mut descriptor: Box<SECURITY_DESCRIPTOR> =
                Box::new(unsafe { std::mem::zeroed() });
            unsafe {
                let descriptor_ptr = &mut *descriptor as *mut _ as *mut core::ffi::c_void;
                if InitializeSecurityDescriptor(descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) == 0
                {
                    return Err(io::Error::last_os_error());
                }
                if SetSecurityDescriptorDacl(descriptor_ptr, 1, acl as *const _, 0) == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(Self {
                _token: token,
                _info: info,
                _acl: AclGuard(acl),
                descriptor,
            })
        }

        fn create(&mut self, name: &str, first: bool) -> io::Result<NamedPipeServer> {
            let attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: &mut *self.descriptor as *mut _ as *mut core::ffi::c_void,
                bInheritHandle: 0,
            };
            create_raw_pipe_instance(name, &attributes, first)
        }
    }

    /// Outcome of one synchronous creation attempt.
    enum CreationAttempt {
        Claimed(NamedPipeServer),
        /// The name already exists (`FIRST_PIPE_INSTANCE` access-denied).
        NameExists,
        Failed(io::Error),
    }

    /// One synchronous creation attempt. All raw-pointer setup lives and
    /// dies inside this sync frame so it can never cross an `.await`
    /// (the claim future is spawned on the runtime and must stay `Send`).
    fn try_create_pipe_instance(name: &str, first: bool) -> CreationAttempt {
        let mut setup = match ClaimSetup::build() {
            Ok(setup) => setup,
            Err(error) => return CreationAttempt::Failed(error),
        };
        match setup.create(name, first) {
            Ok(server) => CreationAttempt::Claimed(server),
            Err(error) if first && error.kind() == io::ErrorKind::PermissionDenied => {
                CreationAttempt::NameExists
            }
            Err(error) => CreationAttempt::Failed(error),
        }
    }

    /// Creates the first pipe instance, proving no live owner for `name`.
    /// A name that already exists is taken over only when no listener
    /// answers a bounded probe (stale handles from a crashed server).
    /// The user-only DACL is attached at creation: the descriptor belongs
    /// to the pipe name, so this one call covers every later instance.
    pub async fn claim_pipe_instance(name: &str) -> io::Result<NamedPipeServer> {
        match try_create_pipe_instance(name, true) {
            CreationAttempt::Claimed(server) => return Ok(server),
            CreationAttempt::Failed(error) => return Err(error),
            CreationAttempt::NameExists => {}
        }
        if probe_live_pipe_server(name).await {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "a TikTools control host already owns the local IPC endpoint",
            ));
        }
        tracing::warn!(
            pipe = name,
            "taking over a control pipe name with no live listener"
        );
        // The descriptor argument is ignored for an existing name: it
        // keeps the DACL its creator attached.
        match try_create_pipe_instance(name, false) {
            CreationAttempt::Claimed(server) => Ok(server),
            CreationAttempt::NameExists => Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "a TikTools control host already owns the local IPC endpoint",
            )),
            CreationAttempt::Failed(error) => Err(error),
        }
    }

    /// Raw first-instance creation with an explicit security descriptor
    /// (tokio's `ServerOptions` cannot carry one). Flags mirror tokio's
    /// byte-mode duplex server plus overlapped I/O for the runtime.
    fn create_raw_pipe_instance(
        name: &str,
        attributes: &SECURITY_ATTRIBUTES,
        first: bool,
    ) -> io::Result<NamedPipeServer> {
        let mut open_mode = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
        if first {
            open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
        }
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                open_mode,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                65_536,
                65_536,
                0,
                attributes,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // Sole ownership of a fresh overlapped handle, inside the runtime:
        // the documented `from_raw_handle` contract.
        unsafe { NamedPipeServer::from_raw_handle(handle) }
    }

    /// True when a listener accepts on `name`. Retried briefly because a
    /// live server recreates its listening instance right after every
    /// accept; persistent failure means stale handles, not a live owner.
    /// Access-denied fails closed (treated as live): our own pipe must
    /// always admit us, so a denial means someone else's server.
    pub async fn probe_live_pipe_server(name: &str) -> bool {
        use tokio::net::windows::named_pipe::ClientOptions;
        for _ in 0..5 {
            match ClientOptions::new().open(name) {
                Ok(_) => return true,
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => return true,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
            }
        }
        false
    }
}

async fn serve_stream<R, W>(
    api: &ControlApi,
    mut reader: R,
    mut writer: W,
    stream_events: bool,
) -> std::io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut events = stream_events.then(|| api.subscribe());
    let mut line = Vec::with_capacity(4096);
    loop {
        tokio::select! {
            biased;
            result = read_capped_line(&mut reader, &mut line) => {
                match result? {
                    None => return Ok(()),
                    Some(LineOutcome::Ok) => {
                        let response = execute_line(api, &line).await;
                        line.clear();
                        write_response(&mut writer, &response).await?;
                        // Flush events the request published (a biased select
                        // would otherwise starve them behind piped requests).
                        drain_events(&mut writer, &mut events).await?;
                        if api.core().is_shutdown() {
                            writer.flush().await?;
                            return Ok(());
                        }
                    }
                    Some(LineOutcome::TooLarge) => {
                        // `line` holds the recovery prefix: preserve the
                        // request id so the caller fails fast with
                        // `too_large` instead of timing out.
                        let id = crate::RpcId::extract_from_prefix(
                            &String::from_utf8_lossy(line.as_slice()),
                        );
                        line.clear();
                        let response = RpcResponse::error(id, ApiError::too_large());
                        write_response(&mut writer, &response).await?;
                    }
                }
            }
            event = async {
                match events.as_mut() {
                    Some(receiver) => receiver.recv().await.ok(),
                    None => std::future::pending().await,
                }
            } => {
                if let Some(event) = event {
                    let notification = event_notification(&event);
                    let mut bytes = serde_json::to_vec(&notification)
                        .unwrap_or_else(|_| b"{}".to_vec());
                    bytes.push(b'\n');
                    writer.write_all(&bytes).await?;
                    writer.flush().await?;
                }
            }
        }
    }
}

enum LineOutcome {
    Ok,
    TooLarge,
}

/// Bytes of an overlong line kept for request-id recovery, so the
/// `too_large` error still correlates with the pending call instead of
/// hanging it until timeout. Must cover the `{"jsonrpc","id",...}` prefix.
const ID_PREFIX_RECOVERY_BYTES: usize = 256;

/// Reads one `\n`-terminated line capped at [`MAX_REQUEST_BYTES`]. Overlong
/// lines are discarded through the terminator so framing stays aligned,
/// keeping only an id-recovery prefix in `line`.
async fn read_capped_line<R>(
    reader: &mut R,
    line: &mut Vec<u8>,
) -> std::io::Result<Option<LineOutcome>>
where
    R: AsyncBufRead + Unpin,
{
    line.clear();
    let mut total = 0usize;
    let mut too_large = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok((!too_large && total > 0).then_some(LineOutcome::Ok));
        }
        let end = chunk.iter().position(|byte| *byte == b'\n');
        match end {
            Some(index) => {
                let take = index + 1;
                if !too_large {
                    total += take;
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        // Keep the line prefix for id recovery even when a
                        // single chunk blows the cap with `line` still empty.
                        let room = ID_PREFIX_RECOVERY_BYTES.saturating_sub(line.len());
                        line.extend_from_slice(&chunk[..take.min(room)]);
                        line.truncate(ID_PREFIX_RECOVERY_BYTES);
                    } else {
                        line.extend_from_slice(&chunk[..take]);
                    }
                }
                reader.consume(take);
                return Ok(Some(if too_large {
                    LineOutcome::TooLarge
                } else {
                    LineOutcome::Ok
                }));
            }
            None => {
                if !too_large {
                    total += chunk.len();
                    if total > MAX_REQUEST_BYTES {
                        too_large = true;
                        let room = ID_PREFIX_RECOVERY_BYTES.saturating_sub(line.len());
                        line.extend_from_slice(&chunk[..chunk.len().min(room)]);
                        line.truncate(ID_PREFIX_RECOVERY_BYTES);
                    } else {
                        line.extend_from_slice(chunk);
                    }
                }
                let len = chunk.len();
                reader.consume(len);
            }
        }
    }
}

/// Writes every queued domain event as a notification. Stops at the
/// first empty/lagged/closed drain so a hot bus cannot stall responses.
async fn drain_events<W>(
    writer: &mut W,
    events: &mut Option<tiktools_core::events::DomainSubscription>,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let Some(receiver) = events.as_mut() else {
        return Ok(());
    };
    while let Ok(event) = receiver.try_recv() {
        let notification = event_notification(&event);
        let mut bytes = serde_json::to_vec(&notification).unwrap_or_else(|_| b"{}".to_vec());
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
    }
    writer.flush().await
}

async fn execute_line(api: &ControlApi, line: &[u8]) -> RpcResponse {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return RpcResponse::error(
            crate::RpcId::Null,
            ApiError::invalid_params("empty request"),
        );
    }
    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(raw) => api.execute_value(&raw).await,
        Err(error) => RpcResponse::error(
            crate::RpcId::extract_from_prefix(trimmed),
            ApiError::invalid_params(format!("invalid JSON: {error}")),
        ),
    }
}

async fn write_response<W>(writer: &mut W, response: &RpcResponse) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut bytes = serde_json::to_vec(response)
        .unwrap_or_else(|_| b"{\"jsonrpc\":\"2.0\",\"id\":null}".to_vec());
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives one request plus one domain event through the NDJSON framing
    /// over an in-memory duplex (no sockets needed).
    #[tokio::test]
    async fn ndjson_framing_carries_responses_and_events() {
        struct Emitter;
        impl tiktools_core::HostEmitter for Emitter {
            fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
        }
        // Isolated home so the core never touches real user data.
        let home = std::env::temp_dir().join(format!(
            "tiktools-transport-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::env::set_var("TIKTOOLS_HOME", &home);
        let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
        let api = ControlApi::new(std::sync::Arc::clone(&core));
        let (client, server) = tokio::io::duplex(64 * 1024);
        let (server_read, server_write) = tokio::io::split(server);
        let server_task = tokio::spawn(async move {
            serve_stream(
                &api,
                tokio::io::BufReader::new(server_read),
                server_write,
                true,
            )
            .await
        });
        let (client_read, mut client_write) = tokio::io::split(client);
        client_write
            .write_all(b"{\"id\":1,\"method\":\"system.ping\"}\n")
            .await
            .unwrap();
        let mut lines = tokio::io::BufReader::new(client_read).lines();
        // Every wait below is bounded: a stuck server must fail the test,
        // never hang the suite.
        let response = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("response line timed out")
            .unwrap()
            .expect("response line");
        assert!(
            response.contains("\"id\":1"),
            "unexpected response: {response}"
        );
        assert!(
            response.contains("\"ok\":true"),
            "unexpected response: {response}"
        );
        // The server is now inside its select loop (hence subscribed), so a
        // domain event published here interleaves as a JSON-RPC notification.
        core.events
            .publish_domain(tiktools_core::events::DomainEvent::LiveDisconnected);
        let event = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("event line timed out")
            .unwrap()
            .expect("event line");
        assert!(
            event.contains("\"method\":\"event\""),
            "unexpected event: {event}"
        );
        assert!(
            event.contains("live.disconnected"),
            "unexpected event: {event}"
        );
        // Split halves share the duplex endpoint, so a bare drop would not
        // deliver EOF; an explicit shutdown closes the write side.
        client_write.shutdown().await.unwrap();
        let eof = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("EOF timed out")
            .unwrap();
        assert!(eof.is_none());
        tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
            .await
            .expect("server task hung after EOF")
            .expect("server task panicked")
            .unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[tokio::test]
    async fn overlong_lines_error_without_breaking_framing() {
        let input = format!(
            "{{\"id\":1,\"method\":\"{}}}\n{{\"id\":2,\"method\":\"system.ping\"}}\n",
            "x".repeat(MAX_REQUEST_BYTES)
        );
        let mut reader = tokio::io::BufReader::new(input.as_bytes());
        let mut line = Vec::new();
        assert!(matches!(
            read_capped_line(&mut reader, &mut line).await.unwrap(),
            Some(LineOutcome::TooLarge)
        ));
        // The recovery prefix survives so the `too_large` error carries id 1.
        assert_eq!(
            crate::RpcId::extract_from_prefix(&String::from_utf8_lossy(&line)),
            crate::RpcId::Number(1)
        );
        assert!(matches!(
            read_capped_line(&mut reader, &mut line).await.unwrap(),
            Some(LineOutcome::Ok)
        ));
        assert!(String::from_utf8_lossy(&line).contains("\"id\":2"));
    }

    #[tokio::test]
    async fn oversized_request_errors_with_its_id_and_framing_survives() {
        struct Emitter;
        impl tiktools_core::HostEmitter for Emitter {
            fn emit(&self, _message: tiktools_core::ipc::messages::HostMessage) {}
        }
        let home = std::env::temp_dir().join(format!(
            "tiktools-transport-oversized-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        std::env::set_var("TIKTOOLS_HOME", &home);
        let core = std::sync::Arc::new(tiktools_core::AppCore::new(std::sync::Arc::new(Emitter)));
        let api = ControlApi::new(std::sync::Arc::clone(&core));
        let (client, server) = tokio::io::duplex(4 * 1024 * 1024);
        let (server_read, server_write) = tokio::io::split(server);
        let server_task = tokio::spawn(async move {
            serve_stream(
                &api,
                tokio::io::BufReader::new(server_read),
                server_write,
                false,
            )
            .await
        });
        let (client_read, mut client_write) = tokio::io::split(client);
        let oversized = format!(
            "{{\"id\":9,\"method\":\"system.ping\",\"params\":{{\"blob\":\"{}\"}}}}\n",
            "y".repeat(MAX_REQUEST_BYTES)
        );
        client_write.write_all(oversized.as_bytes()).await.unwrap();
        client_write
            .write_all(b"{\"id\":10,\"method\":\"system.ping\"}\n")
            .await
            .unwrap();
        let mut lines = tokio::io::BufReader::new(client_read).lines();
        let first = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("too_large line timed out")
            .unwrap()
            .expect("too_large line");
        assert!(
            first.contains("\"id\":9") && first.contains("too_large"),
            "oversized error must carry its id: {first}"
        );
        let second = tokio::time::timeout(std::time::Duration::from_secs(10), lines.next_line())
            .await
            .expect("follow-up line timed out")
            .unwrap()
            .expect("follow-up line");
        assert!(
            second.contains("\"id\":10") && second.contains("\"ok\":true"),
            "framing must survive the oversized line: {second}"
        );
        client_write.shutdown().await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), server_task)
            .await
            .expect("server task hung after EOF")
            .expect("server task panicked")
            .unwrap();
        let _ = std::fs::remove_dir_all(&home);
    }

    #[cfg(windows)]
    #[test]
    fn user_sid_string_is_pipe_safe() {
        let sid = super::pipe_security::current_user_sid_string().expect("user SID resolves");
        assert!(sid.starts_with("S-1-"), "unexpected SID string form: {sid}");
        assert!(
            sid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "SID must be pipe-name safe: {sid}"
        );
    }

    #[cfg(windows)]
    #[test]
    fn production_pipe_name_is_per_user_and_override_is_exact() {
        let previous = std::env::var("TIKTOOLS_IPC_NAME").ok();
        std::env::set_var("TIKTOOLS_IPC_NAME", "tiktools-test-pipe-1");
        assert_eq!(
            super::ipc_pipe_name().expect("override name"),
            r"\\.\pipe\tiktools-test-pipe-1"
        );
        std::env::remove_var("TIKTOOLS_IPC_NAME");
        let production = super::ipc_pipe_name().expect("production name");
        assert!(
            production.starts_with(r"\\.\pipe\tiktools-control-S-"),
            "production pipe must carry the user SID: {production}"
        );
        if let Some(value) = previous {
            std::env::set_var("TIKTOOLS_IPC_NAME", value);
        } else {
            std::env::remove_var("TIKTOOLS_IPC_NAME");
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn pipe_probe_sees_listeners_and_claim_refuses_second_owner() {
        use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let name = format!(
            r"\\.\pipe\tiktools-claim-test-{}-{unique}",
            std::process::id()
        );
        // A claimed instance stays connectable under its user-only ACL,
        // and the raw handle interoperates with the runtime.
        let server = super::pipe_security::claim_pipe_instance(&name)
            .await
            .expect("first claim succeeds");
        let _client = ClientOptions::new()
            .open(&name)
            .expect("owner connects under its own ACL");
        tokio::time::timeout(std::time::Duration::from_secs(5), server.connect())
            .await
            .expect("connect must complete")
            .expect("connect succeeds");
        drop(server);
        drop(_client);
        // Emulate the server accept loop: always keep one free listening
        // instance (each probe consumes one; the loop recreates it).
        let loop_name = name.clone();
        let listener = tokio::spawn(async move {
            loop {
                let next = ServerOptions::new()
                    .first_pipe_instance(false)
                    .create(&loop_name)
                    .expect("loop instance");
                let _ = next.connect().await;
            }
        });
        assert!(
            super::pipe_security::probe_live_pipe_server(&name).await,
            "served name must probe live"
        );
        // A second claim for the same name fails instead of splitting
        // clients across two runtimes.
        let rival = super::pipe_security::claim_pipe_instance(&name).await;
        assert!(
            matches!(rival, Err(ref error) if error.kind() == std::io::ErrorKind::AddrInUse),
            "second claim must fail with AddrInUse: {rival:?}"
        );
        listener.abort();
        let _ = listener.await;
        // With every handle closed the name is gone: the probe reports
        // idle and the name is claimable again (crash recovery).
        assert!(
            !super::pipe_security::probe_live_pipe_server(&name).await,
            "released name must probe idle"
        );
        let _reclaimed = super::pipe_security::claim_pipe_instance(&name)
            .await
            .expect("released name is claimable again");
    }
}
