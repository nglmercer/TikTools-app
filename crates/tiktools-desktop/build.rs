//! Embeds the Windows icon and version resource into `tiktools-desktop.exe`.
//!
//! Without this resource Explorer, shortcuts, the NSIS installer's uninstall
//! entry and the file's Details tab show a generic icon and no company or
//! product name. The window and tray icons are drawn at runtime (`icon.rs`),
//! so they were never affected.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/tiktools-icon.ico");

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../../assets/tiktools-icon.ico")
            .set("ProductName", "TikTools")
            .set("FileDescription", "TikTools")
            .set("CompanyName", "TikTools")
            .set("LegalCopyright", "MIT License");
        if let Err(error) = resource.compile() {
            // Fail loudly: a release without the icon is the bug this fixes.
            panic!("failed to embed the Windows icon and version resource: {error}");
        }
    }
}
