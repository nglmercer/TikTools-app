//! Musl-side proof for host binary selection.
//!
//! Built for `x86_64-unknown-linux-musl` and executed by the
//! `musl_target_selects_musl_binary` integration test. A full musl guest
//! load is untestable here (static musl binaries cannot `dlopen`), so this
//! probe covers exactly TikTools' share: libc detection, the napi-rs host
//! spelling, and exact host binary selection from a package root.

use std::path::PathBuf;

use tiktools_plugin_api::manifest::{current_napi_target, is_musl, select_host_native_binary};

fn main() {
    assert!(is_musl(), "expected musl libc, got gnu");
    assert_eq!(current_napi_target(), "linux-x64-musl");
    let root: PathBuf = std::env::temp_dir().join(format!("tiktools-musl-probe-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("node-rdev.linux-x64-gnu.node"), b"gnu").unwrap();
    std::fs::write(root.join("node-rdev.linux-x64-musl.node"), b"musl").unwrap();
    std::fs::write(root.join("node-rdev.win32-x64-msvc.node"), b"foreign").unwrap();
    let selected = select_host_native_binary(&root).expect("musl binary must be selected");
    assert_eq!(selected, root.join("node-rdev.linux-x64-musl.node"));
    std::fs::remove_dir_all(&root).ok();
    println!("MUSL PROBE OK: {}", selected.display());
}
