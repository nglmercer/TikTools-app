pub(crate) const TARGETS: [(&str, &str); 6] = [
    ("win32-x64-msvc", "x86_64-pc-windows-msvc"),
    ("win32-arm64-msvc", "aarch64-pc-windows-msvc"),
    ("linux-x64-gnu", "x86_64-unknown-linux-gnu"),
    ("linux-arm64-gnu", "aarch64-unknown-linux-gnu"),
    ("darwin-x64-darwin", "x86_64-apple-darwin"),
    ("darwin-arm64-darwin", "aarch64-apple-darwin"),
];

pub(crate) fn rust_triple(plugin_target: &str) -> Option<&'static str> {
    TARGETS
        .iter()
        .find(|(target, _)| *target == plugin_target)
        .map(|(_, triple)| *triple)
}

pub(crate) fn host_target() -> String {
    tiktools_plugin_api::manifest::current_target()
}

pub(crate) fn executable_suffix(plugin_target: &str) -> &'static str {
    if plugin_target.starts_with("win32-") {
        ".exe"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_packaged_target_has_a_rust_triple() {
        for target in tiktools_plugin_api::manifest::PLUGIN_TARGETS {
            assert!(super::rust_triple(target).is_some(), "{target}");
        }
        assert_eq!(
            super::TARGETS.len(),
            tiktools_plugin_api::manifest::PLUGIN_TARGETS.len()
        );
    }
}
