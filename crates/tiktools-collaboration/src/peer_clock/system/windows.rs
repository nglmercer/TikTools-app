//! A per-user volatile marker survives app restarts, not OS/hive restarts.
//! Losing this marker requires Cloud validation; wall time is never a fallback.
use std::{ptr, time::Duration};
use windows_sys::Win32::{
    Foundation::ERROR_SUCCESS,
    System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_QUERY_VALUE, KEY_SET_VALUE, KEY_WOW64_64KEY, REG_BINARY, REG_CREATED_NEW_KEY,
        REG_OPTION_VOLATILE,
    },
};

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: only handles returned by successful RegCreateKeyExW are owned.
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

pub(super) fn boot_id() -> Option<String> {
    let path: Vec<u16> = "Volatile Environment\\TikToolsPeerClockV1\0"
        .encode_utf16()
        .collect();
    let name: Vec<u16> = "Identity\0".encode_utf16().collect();
    let mut raw = ptr::null_mut();
    let mut disposition = 0;
    // SAFETY: all strings are NUL terminated and output pointers are writable.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            ptr::null(),
            REG_OPTION_VOLATILE,
            KEY_QUERY_VALUE | KEY_SET_VALUE | KEY_WOW64_64KEY,
            ptr::null(),
            &mut raw,
            &mut disposition,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let key = Key(raw);
    if disposition == REG_CREATED_NEW_KEY {
        let id = uuid::Uuid::new_v4();
        // SAFETY: value points to exactly sixteen readable bytes.
        let status = unsafe {
            RegSetValueExW(
                key.0,
                name.as_ptr(),
                0,
                REG_BINARY,
                id.as_bytes().as_ptr(),
                16,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        return Some(id.to_string());
    }
    // Another process may have opened the newly created key before its owner
    // published the marker. Never replace an existing identity after an error.
    for _ in 0..5 {
        let mut bytes = [0_u8; 16];
        let mut size = bytes.len() as u32;
        let mut kind = 0;
        // SAFETY: size accurately describes the writable fixed-size buffer.
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                name.as_ptr(),
                ptr::null(),
                &mut kind,
                bytes.as_mut_ptr(),
                &mut size,
            )
        };
        if status == ERROR_SUCCESS && size == 16 && kind == REG_BINARY {
            return Some(uuid::Uuid::from_bytes(bytes).to_string());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}
