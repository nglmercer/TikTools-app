//! Musl-side proof for host target detection and artifact selection.
//!
//! Built for `x86_64-unknown-linux-musl` and executed by the
//! `musl_target_selects_musl_artifact` integration test. A full musl
//! guest load is untestable here (static musl binaries cannot `dlopen`),
//! so this probe covers exactly TikTools' share: libc detection, the
//! napi-rs host spelling, and exact artifact selection. The loader half —
//! the generated detector answering the guest-visible report — is covered
//! by napi-vm's own `guest_target` tests with a mocked musl identity.

use tiktools_plugin_api::manifest::{current_napi_target, is_musl};
use tiktools_plugin_api::{NativeAddonArtifact, NativeAddonDeclaration};

fn main() {
    assert!(is_musl(), "expected musl libc, got gnu");
    assert_eq!(current_napi_target(), "linux-x64-musl");
    let declaration = NativeAddonDeclaration {
        package: "pkg".into(),
        root: "node_modules/pkg".into(),
        artifacts: [
            (
                "linux-x64-gnu".to_owned(),
                NativeAddonArtifact {
                    path: "gnu.node".into(),
                    sha256: "a".repeat(64),
                },
            ),
            (
                "linux-x64-musl".to_owned(),
                NativeAddonArtifact {
                    path: "musl.node".into(),
                    sha256: "b".repeat(64),
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    let (key, artifact) = declaration
        .select_host_artifact()
        .expect("musl artifact must be selected");
    assert_eq!(key, "linux-x64-musl");
    assert_eq!(artifact.path, "musl.node");
    println!("MUSL PROBE OK: {key}");
}
