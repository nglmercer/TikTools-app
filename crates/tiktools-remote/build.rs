use std::{env, fs, path::PathBuf};

fn main() {
    if env::var_os("CARGO_FEATURE_BROWSER_APP").is_none() {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../dist/web");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("index.html").display()
    );
    println!("cargo:rerun-if-changed={}", root.join("assets").display());
    let index_path = fs::canonicalize(root.join("index.html"))
        .expect("browser-app requires a built dist/web/index.html");
    let html = fs::read_to_string(&index_path).expect("browser-app index must be UTF-8");
    assert!(
        html.len() <= 1024 * 1024
            && html.matches("<head>").count() == 1
            && !html.contains("tiktools-transport"),
        "invalid browser-app index"
    );
    let assets = root.join("assets");
    let mut entries: Vec<_> = fs::read_dir(&assets)
        .expect("browser-app requires built assets")
        .map(|entry| entry.unwrap())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    let mut generated = format!(
        "pub const INDEX: &str = include_str!({:?});\n",
        index_path.to_str().expect("asset path must be UTF-8")
    );
    generated.push_str(
        "pub fn asset(path: &str) -> Option<(&'static [u8], &'static str)> { match path {\n",
    );
    let mut count = 0usize;
    let mut total = html.len();
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .expect("asset filename must be UTF-8");
        if name.ends_with(".map") {
            continue;
        }
        assert!(
            name.len() <= 200
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')),
            "invalid browser-app filename"
        );
        let mime = match name.rsplit_once('.').map(|(_, ext)| ext) {
            Some("js") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("woff2") => "font/woff2",
            _ => continue,
        };
        let metadata = entry.file_type().expect("asset type");
        assert!(
            metadata.is_file() && !metadata.is_symlink(),
            "browser-app asset must be a regular file"
        );
        let size = entry.metadata().expect("asset metadata").len() as usize;
        assert!(size <= 8 * 1024 * 1024, "browser-app asset too large");
        total = total.checked_add(size).expect("browser-app size overflow");
        assert!(total <= 64 * 1024 * 1024, "browser-app bundle too large");
        let path = fs::canonicalize(entry.path()).expect("asset path");
        println!("cargo:rerun-if-changed={}", path.display());
        generated.push_str(&format!(
            "{:?} => Some((include_bytes!({:?}), {:?})),\n",
            format!("/assets/{name}"),
            path.to_str().expect("asset path must be UTF-8"),
            mime
        ));
        count += 1;
    }
    assert!(
        count > 0 && count <= 2048,
        "invalid browser-app asset count"
    );
    generated.push_str("_ => None,\n} }\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("browser_assets.rs"),
        generated,
    )
    .unwrap();
}
