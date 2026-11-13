use super::Stamp;
use std::sync::OnceLock;

pub(super) fn sample() -> Option<Stamp> {
    // Boot identity initialization can read OS metadata once. Admission checks
    // thereafter use only the cached identity and a cheap OS clock read.
    static BOOT: OnceLock<Option<String>> = OnceLock::new();
    Some(Stamp {
        boot: BOOT.get_or_init(boot_id).clone()?,
        elapsed_ms: elapsed_ms()?,
    })
}

#[cfg(target_os = "linux")]
fn boot_id() -> Option<String> {
    let id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    Some(uuid::Uuid::parse_str(id.trim()).ok()?.to_string())
}

#[cfg(target_os = "linux")]
fn elapsed_ms() -> Option<i64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: the output points to an initialized, correctly sized timespec.
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time) } != 0
        || time.tv_sec < 0
        || !(0..1_000_000_000).contains(&time.tv_nsec)
    {
        return None;
    }
    time.tv_sec
        .checked_mul(1000)?
        .checked_add(time.tv_nsec / 1_000_000)
}

#[cfg(target_os = "macos")]
fn boot_id() -> Option<String> {
    let mut value = [0_u8; 128];
    let mut length = value.len();
    // SAFETY: name is NUL terminated; value/length describe writable storage.
    let result = unsafe {
        libc::sysctlbyname(
            c"kern.bootsessionuuid".as_ptr(),
            value.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || length == 0 || length > value.len() {
        return None;
    }
    let raw = std::str::from_utf8(&value[..length])
        .ok()?
        .trim_end_matches('\0');
    Some(uuid::Uuid::parse_str(raw).ok()?.to_string())
}

#[cfg(target_os = "macos")]
fn elapsed_ms() -> Option<i64> {
    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }
    unsafe extern "C" {
        fn mach_continuous_time() -> u64;
        fn mach_timebase_info(info: *mut Timebase) -> libc::c_int;
    }
    let mut info = Timebase { numer: 0, denom: 0 };
    // SAFETY: info is valid output storage; both APIs have no caller state.
    if unsafe { mach_timebase_info(&mut info) } != 0 || info.denom == 0 {
        return None;
    }
    let ticks = unsafe { mach_continuous_time() };
    i64::try_from(
        u128::from(ticks).checked_mul(u128::from(info.numer))? / u128::from(info.denom) / 1_000_000,
    )
    .ok()
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::boot_id;
#[cfg(windows)]
fn elapsed_ms() -> Option<i64> {
    // SAFETY: GetTickCount64 has no pointers or caller state and includes sleep.
    i64::try_from(unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }).ok()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn boot_id() -> Option<String> {
    None
}
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn elapsed_ms() -> Option<i64> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    fn boot_clock_is_available_and_monotonic() {
        let first = super::sample().expect("supported OS must expose a boot clock");
        let second = super::sample().unwrap();
        assert_eq!(first.boot, second.boot);
        assert!(second.elapsed_ms >= first.elapsed_ms);
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    fn boot_identity_survives_a_process_restart() {
        let sample = super::sample().unwrap();
        if let Ok(expected) = std::env::var("TIKTOOLS_TEST_EXPECT_BOOT_ID") {
            assert_eq!(sample.boot, expected);
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "peer_clock::system::tests::boot_identity_survives_a_process_restart",
                "--nocapture",
            ])
            .env("TIKTOOLS_TEST_EXPECT_BOOT_ID", sample.boot)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
