//! Single-owner guard for the production control IPC endpoint.
//!
//! Only one control server may own the well-known endpoint at a time. The
//! guard is an OS ownership primitive held for the server's lifetime — never
//! a `ControlClient::connect()` probe, which races with startup and cannot
//! distinguish a live owner from a stale socket.

use std::io;

/// Well-known mutex name for the production control host (Windows).
pub const CONTROL_HOST_MUTEX: &str = "Local\\TikTools.ControlHost";

/// Acquires exclusive ownership of the control IPC endpoint. Fails with
/// `AddrInUse` when another host already owns it.
pub fn acquire_control_host_ownership() -> io::Result<ControlHostGuard> {
    platform::acquire()
}

/// Name of the ownership primitive for the current endpoint. Test runs that
/// override `TIKTOOLS_IPC_NAME` get an isolated mutex so they never contend
/// with the production host.
pub fn ownership_name() -> String {
    platform::ownership_name()
}

pub struct ControlHostGuard {
    #[allow(dead_code)]
    platform: platform::PlatformGuard,
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ptr;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };

    pub struct PlatformGuard {
        mutex: usize,
    }

    // The mutex handle is owned by this guard and only touched on the
    // thread that drops it; the guard itself is moved across tasks.
    unsafe impl Send for PlatformGuard {}
    unsafe impl Sync for PlatformGuard {}

    pub fn ownership_name() -> String {
        if let Ok(name) = std::env::var("TIKTOOLS_IPC_NAME") {
            if !name.is_empty()
                && name.len() <= 64
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return format!("{}.{name}", super::CONTROL_HOST_MUTEX);
            }
        }
        super::CONTROL_HOST_MUTEX.to_owned()
    }

    pub fn acquire() -> io::Result<super::ControlHostGuard> {
        let name = wide(&ownership_name());
        // Initial ownership is requested but never waited on: the mutex is
        // purely an existence primitive, so abandonment is impossible.
        let mutex = unsafe { CreateMutexW(ptr::null(), 1, name.as_ptr()) };
        if mutex.is_null() {
            return Err(io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(mutex) };
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "a TikTools control host already owns the local IPC endpoint",
            ));
        }
        Ok(super::ControlHostGuard {
            platform: PlatformGuard {
                mutex: mutex as usize,
            },
        })
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    impl Drop for PlatformGuard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.mutex as windows_sys::Win32::Foundation::HANDLE);
            }
        }
    }
}

#[cfg(unix)]
mod platform {
    use super::*;

    pub struct PlatformGuard {
        _private: (),
    }

    pub fn ownership_name() -> String {
        super::CONTROL_HOST_MUTEX.to_owned()
    }

    pub fn acquire() -> io::Result<super::ControlHostGuard> {
        // On Unix the socket bind itself is the ownership primitive:
        // `run_ipc_unix` only unlinks a stale path after a failed connect,
        // so a live owner makes the bind fail with `AddrInUse`.
        Ok(super::ControlHostGuard {
            platform: PlatformGuard { _private: () },
        })
    }
}

#[cfg(not(any(windows, unix)))]
mod platform {
    use super::*;

    pub struct PlatformGuard {
        _private: (),
    }

    pub fn ownership_name() -> String {
        super::CONTROL_HOST_MUTEX.to_owned()
    }

    pub fn acquire() -> io::Result<super::ControlHostGuard> {
        Ok(super::ControlHostGuard {
            platform: PlatformGuard { _private: () },
        })
    }
}
