//! One-click raw-input access for Linux/Wayland global hotkeys.
//!
//! The Wayland evdev backend reads physical keyboards through
//! `/dev/input/event*`, which needs either `input` group membership or a
//! logind seat ACL. Instead of terminal commands, the UI offers a "grant
//! access" button backed by [`request_input_access`]: it probes the
//! current readability and, when blocked, installs the same seat rule the
//! `rdev-node` setup script ships ([`UDEV_RULE_BODY`]) through a single
//! polkit prompt (`pkexec`, no terminal). Already-privileged hosts (root,
//! containers) install directly with no prompt at all.
//!
//! No code path here can conjure access without one privileged consent:
//! logind `TakeDevice` is restricted to the session controller (the
//! compositor), and there is no portal for global key capture. The
//! privileged step below is deliberate, minimal, and fully static: root
//! executes a fixed script with absolute paths only, never user input.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Destination of the installed seat rule (same path the rdev-node setup
/// script uses, so both flows converge instead of fighting).
pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/70-rdev-node-input.rules";

/// Rule body. Must stay identical to `rdev-node/udev/70-rdev-node-input.rules`
/// (comments aside): both grant the seat-active user evdev capture plus
/// uinput injection through logind `uaccess` (which also implies the seat
/// tag via systemd's own `71-seat.rules`).
pub const UDEV_RULE_BODY: &str = "SUBSYSTEM==\"input\", KERNEL==\"event*\", TAG+=\"uaccess\"\nKERNEL==\"uinput\", TAG+=\"uaccess\"\n";

/// Fixed absolute locations accepted for the privileged helpers. The
/// escalated script never consults `PATH`: every binary root executes is
/// resolved here, unprivileged, against this allowlist.
#[cfg(target_os = "linux")]
const UDEVADM_CANDIDATES: &[&str] = &["/usr/bin/udevadm", "/sbin/udevadm", "/bin/udevadm"];
#[cfg(target_os = "linux")]
const CHMOD_CANDIDATES: &[&str] = &["/bin/chmod", "/usr/bin/chmod"];
#[cfg(target_os = "linux")]
const SHELL_PATH: &str = "/bin/sh";

/// Outcome of [`request_input_access`]: whether capture is usable now,
/// plus a human sentence for the UI (success) or the failure detail.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputAccessResult {
    pub granted: bool,
    pub message: String,
}

impl InputAccessResult {
    fn granted(message: impl Into<String>) -> Self {
        Self {
            granted: true,
            message: message.into(),
        }
    }

    fn denied(message: impl Into<String>) -> Self {
        Self {
            granted: false,
            message: message.into(),
        }
    }
}

/// Probes and, when blocked, requests raw-input access. Never throws:
/// every failure mode (unsupported platform, missing polkit, dismissed
/// prompt, still denied) comes back as a denied result with a message.
///
/// Linux runs the probe-then-escalate flow; other platforms have no seat
/// rule to install, so there is nothing to do.
pub fn request_input_access() -> InputAccessResult {
    #[cfg(target_os = "linux")]
    {
        request_with(
            &has_raw_input_access,
            &RealRunner,
            &std::env::var("PATH").unwrap_or_default(),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        InputAccessResult::granted(
            "Raw-input seat access is only needed on Linux; nothing to do here.",
        )
    }
}

/// First `/dev/input/event*` node, if the directory exists at all.
#[cfg(target_os = "linux")]
fn first_event_device_in(dev_input: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dev_input).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("event")
                        && name["event".len()..].chars().all(|c| c.is_ascii_digit())
                })
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

#[cfg(target_os = "linux")]
fn is_readable(path: &Path) -> bool {
    std::fs::File::open(path).is_ok()
}

#[cfg(target_os = "linux")]
fn has_raw_input_access() -> bool {
    first_event_device_in(Path::new("/dev/input")).is_some_and(|device| is_readable(&device))
}

/// Absolute-path lookup for a helper inside one of the fixed candidate
/// locations. Used for binaries root will execute; never a `PATH` search.
#[cfg(target_os = "linux")]
fn find_helper(candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

/// `PATH` lookup for the escalation binary itself. `pkexec` runs with the
/// caller's own privileges up to the auth prompt, so resolving it through
/// `PATH` grants nothing extra; everything it is asked to run stays on
/// the absolute-path allowlist above.
#[cfg(target_os = "linux")]
fn find_on_path(name: &str, path_var: &str) -> Option<PathBuf> {
    std::env::split_paths(path_var)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

/// The privileged installer: fixed text, absolute paths, no variables.
/// Idempotent (rewrites the same file, reloads, retriggers), so a retry
/// after a partial failure converges instead of corrupting.
#[cfg(target_os = "linux")]
fn install_script(udevadm: &Path, chmod: &Path) -> String {
    // One single-quoted arg per rule line: no embedded newlines, so every
    // physical line below is one command (the absolute-path test leans on
    // this, as does readability of the polkit-visible command).
    let rules = UDEV_RULE_BODY
        .lines()
        .map(|line| format!("'{line}'"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "set -e\nprintf '%s\\n' {rules} > {UDEV_RULE_PATH}\n{} 0644 {UDEV_RULE_PATH}\n{} control --reload-rules\n{} trigger --subsystem-match=input --action=change\n{} trigger --name-match=uinput --action=change || true\n{} settle --timeout=10 || true\n",
        chmod.display(),
        udevadm.display(),
        udevadm.display(),
        udevadm.display(),
        udevadm.display(),
    )
}

#[cfg(target_os = "linux")]
fn truncate_tail(text: &str, max_chars: usize) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() > max_chars {
        let kept: String = single_line.chars().take(max_chars).collect();
        format!("{kept}…")
    } else {
        single_line
    }
}

#[cfg(target_os = "linux")]
trait Runner {
    fn run(&self, program: &Path, args: &[&str]) -> std::io::Result<std::process::Output>;
}

#[cfg(target_os = "linux")]
struct RealRunner;

#[cfg(target_os = "linux")]
impl Runner for RealRunner {
    fn run(&self, program: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
        std::process::Command::new(program).args(args).output()
    }
}

#[cfg(target_os = "linux")]
fn request_with(
    probe: &dyn Fn() -> bool,
    runner: &dyn Runner,
    path_var: &str,
) -> InputAccessResult {
    if probe() {
        return InputAccessResult::granted("Raw-input access is already available.");
    }
    let (Some(udevadm), Some(chmod)) = (
        find_helper(UDEVADM_CANDIDATES),
        find_helper(CHMOD_CANDIDATES),
    ) else {
        return InputAccessResult::denied(
            "udevadm or chmod is missing; install systemd-udev and coreutils first.",
        );
    };
    let script = install_script(&udevadm, &chmod);
    // Fast path: already privileged (root, containers) installs with no
    // prompt. Unprivileged callers fail here instantly at the redirect
    // and fall through to the polkit prompt below.
    if runner
        .run(Path::new(SHELL_PATH), &["-c", script.as_str()])
        .is_ok_and(|output| output.status.success())
        && probe()
    {
        return InputAccessResult::granted("Raw-input access granted (already privileged).");
    }
    let Some(pkexec) = find_on_path("pkexec", path_var) else {
        return InputAccessResult::denied(
            "Polkit (pkexec) is not available; run the rdev-node setup-linux-input.sh script with sudo instead.",
        );
    };
    match runner.run(&pkexec, &["/bin/sh", "-c", script.as_str()]) {
        Ok(output) if output.status.success() => {
            if probe() {
                InputAccessResult::granted("Raw-input access granted.")
            } else {
                InputAccessResult::denied(
                    "The rule installed but the devices are still unreadable; is this the active seat? (loginctl session-status)",
                )
            }
        }
        Ok(output) => InputAccessResult::denied(format!(
            "Authorization failed or was dismissed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            truncate_tail(&String::from_utf8_lossy(&output.stderr), 240),
        )),
        Err(error) => InputAccessResult::denied(format!("Could not launch pkexec: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_body_matches_the_shipped_udev_rule() {
        // The privileged installer must converge with the script flow:
        // same devices, same tag. (The script file adds comments only.)
        assert!(UDEV_RULE_BODY.contains("SUBSYSTEM==\"input\""));
        assert!(UDEV_RULE_BODY.contains("KERNEL==\"event*\""));
        assert!(UDEV_RULE_BODY.contains("KERNEL==\"uinput\""));
        assert!(UDEV_RULE_BODY.contains("TAG+=\"uaccess\""));
        assert_eq!(UDEV_RULE_PATH, "/etc/udev/rules.d/70-rdev-node-input.rules");
        // Single-quote-free: each rule line embeds as one shell arg.
        assert!(!UDEV_RULE_BODY.contains('\''));
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;
        use std::sync::Mutex;

        #[test]
        fn install_script_uses_absolute_paths_only() {
            let script = install_script(Path::new("/usr/bin/udevadm"), Path::new("/bin/chmod"));
            for token in ["/usr/bin/udevadm", "/bin/chmod", UDEV_RULE_PATH] {
                assert!(script.contains(token), "{script}");
            }
            assert!(script.contains("reload-rules"), "{script}");
            assert!(script.contains("settle"), "{script}");
            // Every command root executes is absolute-pathed (builtins
            // excepted): nothing resolves through root's PATH.
            for line in script
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                if line == "set -e" || line.starts_with("printf ") {
                    continue;
                }
                assert!(
                    line.starts_with('/'),
                    "privileged line must use an absolute path: {line}"
                );
            }
        }

        #[test]
        fn install_script_is_valid_shell() {
            let script = install_script(Path::new("/usr/bin/udevadm"), Path::new("/bin/chmod"));
            let mut child = std::process::Command::new("/bin/sh")
                .args(["-n"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("sh -n must run on Linux");
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(script.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "installer is not valid shell: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        #[test]
        fn installer_payload_writes_the_exact_rule_body() {
            // Runs only the printf line, redirected at a temp file: the
            // installed bytes must equal the rule const byte-for-byte.
            let script = install_script(Path::new("/usr/bin/udevadm"), Path::new("/bin/chmod"));
            let printf_line = script
                .lines()
                .find(|line| line.starts_with("printf "))
                .expect("installer starts with a printf line");
            let out = std::env::temp_dir()
                .join(format!("tiktools-input-access-rule-{}", std::process::id()));
            let adapted = printf_line.replace(UDEV_RULE_PATH, out.to_str().unwrap());
            let status = std::process::Command::new("/bin/sh")
                .args(["-c", &adapted])
                .status()
                .unwrap();
            assert!(status.success());
            assert_eq!(std::fs::read_to_string(&out).unwrap(), UDEV_RULE_BODY);
            let _ = std::fs::remove_file(&out);
        }

        #[test]
        fn first_event_device_picks_sorted_event_nodes() {
            let directory = std::env::temp_dir().join(format!(
                "tiktools-input-access-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or_default()
            ));
            std::fs::create_dir_all(&directory).unwrap();
            for name in ["event10", "mice", "event2", "mouse0"] {
                std::fs::write(directory.join(name), b"x").unwrap();
            }
            // Lexicographic, like the evdev backend's device scan.
            assert_eq!(
                first_event_device_in(&directory).as_deref(),
                Some(directory.join("event10").as_path())
            );
            assert!(is_readable(&directory.join("event2")));
            assert!(!is_readable(&directory.join("event-missing")));
            let _ = std::fs::remove_dir_all(&directory);
        }

        #[test]
        fn path_lookup_finds_nothing_outside_path() {
            let directory = std::env::temp_dir()
                .join(format!("tiktools-input-access-path-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).unwrap();
            assert_eq!(find_on_path("pkexec", directory.to_str().unwrap()), None);
            std::fs::write(directory.join("pkexec"), b"x").unwrap();
            assert_eq!(
                find_on_path("pkexec", directory.to_str().unwrap()),
                Some(directory.join("pkexec"))
            );
            let _ = std::fs::remove_dir_all(&directory);
        }

        #[test]
        fn tail_truncation_collapses_whitespace() {
            assert_eq!(truncate_tail("  a\nb\tc  ", 100), "a b c");
            assert_eq!(truncate_tail("abcdef", 4), "abcd…");
        }

        struct StubRunner {
            calls: Mutex<Vec<(PathBuf, Vec<String>)>>,
            direct: std::io::Result<bool>,
            pkexec: std::io::Result<std::process::Output>,
        }

        impl Runner for StubRunner {
            fn run(&self, program: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
                self.calls.lock().unwrap().push((
                    program.to_path_buf(),
                    args.iter().map(ToString::to_string).collect(),
                ));
                if program == Path::new(SHELL_PATH) {
                    return self
                        .direct
                        .as_ref()
                        .map(|success| {
                            let mut command = std::process::Command::new("true");
                            if !success {
                                command = std::process::Command::new("false");
                            }
                            command.output().unwrap()
                        })
                        .map_err(|error| std::io::Error::new(error.kind(), "stub"));
                }
                match &self.pkexec {
                    Ok(output) => Ok(std::process::Output {
                        status: output.status,
                        stdout: output.stdout.clone(),
                        stderr: output.stderr.clone(),
                    }),
                    Err(error) => Err(std::io::Error::new(error.kind(), "stub")),
                }
            }
        }

        fn failing_output(code: i32, stderr: &str) -> std::process::Output {
            use std::os::unix::process::ExitStatusExt;
            std::process::Output {
                status: ExitStatusExt::from_raw(code << 8),
                stdout: Vec::new(),
                stderr: stderr.as_bytes().to_vec(),
            }
        }

        #[test]
        fn already_granted_access_short_circuits() {
            let runner = StubRunner {
                calls: Mutex::new(Vec::new()),
                direct: Ok(true),
                pkexec: Err(std::io::Error::other("must not run")),
            };
            let result = request_with(&|| true, &runner, "");
            assert!(result.granted);
            assert!(runner.calls.lock().unwrap().is_empty());
        }

        #[test]
        fn missing_pkexec_denies_without_touching_the_system() {
            let runner = StubRunner {
                calls: Mutex::new(Vec::new()),
                direct: Err(std::io::Error::other("permission denied")),
                pkexec: Err(std::io::Error::other("must not run")),
            };
            let result = request_with(&|| false, &runner, "/nonexistent-path-dir");
            assert!(!result.granted);
            assert!(result.message.contains("pkexec"), "{}", result.message);
            // Only the direct attempt ran; no escalation without pkexec.
            assert_eq!(runner.calls.lock().unwrap().len(), 1);
        }

        #[test]
        fn privileged_direct_install_grants() {
            let calls = std::sync::atomic::AtomicU32::new(0);
            let probe = || calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0;
            let runner = StubRunner {
                calls: Mutex::new(Vec::new()),
                direct: Ok(true),
                pkexec: Err(std::io::Error::other("must not run")),
            };
            let result = request_with(&probe, &runner, "");
            assert!(result.granted, "{}", result.message);
            assert!(
                result.message.contains("already privileged"),
                "{}",
                result.message
            );
        }

        #[test]
        fn dismissed_prompt_reports_exit_code_and_detail() {
            let runner = StubRunner {
                calls: Mutex::new(Vec::new()),
                direct: Err(std::io::Error::other("permission denied")),
                pkexec: Ok(failing_output(126, "  Request\ndismissed\n")),
            };
            // A fake pkexec on PATH so the flow reaches the prompt branch.
            let directory = std::env::temp_dir().join(format!(
                "tiktools-input-access-pkexec-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join("pkexec"), b"x").unwrap();
            let result = request_with(&|| false, &runner, directory.to_str().unwrap());
            assert!(!result.granted);
            assert!(result.message.contains("126"), "{}", result.message);
            assert!(
                result.message.contains("Request dismissed"),
                "{}",
                result.message
            );
            let _ = std::fs::remove_dir_all(&directory);
        }
    }
}
