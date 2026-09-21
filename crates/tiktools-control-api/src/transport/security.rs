//! Windows pipe ownership and isolation. The pipe namespace is
//! machine-wide, so ownership is enforced here at bind time, layered over
//! the session-scoped ownership mutex (which stays as a fast same-session
//! guard and test hook):
//!
//! * the production pipe name carries the current user SID, so two users
//!   never share a pipe;
//! * the first instance is created with `first_pipe_instance`, so a second
//!   server for the same name fails instead of splitting clients across
//!   two runtimes;
//! * the pipe DACL grants access to the current user only.
//!
//! A name that already exists with no live listener (crashed server whose
//! name is kept alive by stale client handles) is taken over after a
//! bounded live probe, so crash recovery is never bricked.

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
    GetSidIdentifierAuthority, GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation,
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, TokenUser, NO_INHERITANCE,
    SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER,
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
        let mut descriptor: Box<SECURITY_DESCRIPTOR> = Box::new(unsafe { std::mem::zeroed() });
        unsafe {
            let descriptor_ptr = &mut *descriptor as *mut _ as *mut core::ffi::c_void;
            if InitializeSecurityDescriptor(descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) == 0 {
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
