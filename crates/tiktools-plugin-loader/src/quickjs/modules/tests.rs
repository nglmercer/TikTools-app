use super::*;

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tiktools-quickjs-modules-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::canonicalize(root).unwrap()
}

fn shared(root: &Path) -> Rc<HostShared> {
    HostShared::new(
        "test".to_owned(),
        root.to_owned(),
        HashMap::new(),
        None,
        std::sync::Arc::new(|| {}),
    )
}

#[test]
fn relative_resolution_probes_extensions_and_stays_inside_the_root() {
    let root = temp_root("relative");
    fs::create_dir_all(root.join("dist")).unwrap();
    fs::write(root.join("dist/a.js"), "").unwrap();
    fs::write(root.join("dist/data.json"), "{}").unwrap();
    fs::write(root.join("outside.js"), "").unwrap();
    let inner = root.join("dist");
    let shared = shared(&inner);
    let resolved = resolve_specifier(&shared, &inner, "./a", Mode::Import).unwrap();
    assert_eq!(resolved, Resolved::File(inner.join("a.js")));
    assert!(matches!(
        resolve_specifier(&shared, &inner, "./data.json", Mode::Import),
        Ok(Resolved::File(_))
    ));
    let error = resolve_specifier(&shared, &inner, "../outside.js", Mode::Import).unwrap_err();
    assert!(error.contains("escapes"), "{error}");
    let error = resolve_specifier(&shared, &inner, "./missing", Mode::Import).unwrap_err();
    assert!(error.contains("Cannot find module"), "{error}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn builtins_are_deny_by_default() {
    let root = temp_root("builtins");
    let shared = shared(&root);
    assert_eq!(
        resolve_specifier(&shared, &root, "node:module", Mode::Import),
        Ok(Resolved::Builtin(NODE_MODULE))
    );
    for denied in ["node:fs", "node:child_process", "node:path"] {
        assert!(
            resolve_specifier(&shared, &root, denied, Mode::Import).is_err(),
            "{denied}"
        );
    }
    let error = resolve_specifier(&shared, &root, EVENTS_CAPABILITY, Mode::Import).unwrap_err();
    assert!(error.contains("not granted"), "{error}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn packages_resolve_through_exports_main_and_index() {
    let root = temp_root("packages");
    let pkg = root.join("node_modules/pkg");
    fs::create_dir_all(pkg.join("lib")).unwrap();
    fs::write(
        pkg.join("package.json"),
        r#"{"exports":{".":{"import":"./lib/esm.js","require":"./lib/cjs.js"},"./sub":"./lib/sub.js"}}"#,
    )
    .unwrap();
    for file in ["esm.js", "cjs.js", "sub.js"] {
        fs::write(pkg.join("lib").join(file), "").unwrap();
    }
    let plain = root.join("node_modules/plain");
    fs::create_dir_all(&plain).unwrap();
    fs::write(plain.join("package.json"), r#"{"main":"entry"}"#).unwrap();
    fs::write(plain.join("entry.js"), "").unwrap();
    let shared = shared(&root);
    assert_eq!(
        resolve_specifier(&shared, &root, "pkg", Mode::Import).unwrap(),
        Resolved::File(pkg.join("lib/esm.js"))
    );
    assert_eq!(
        resolve_specifier(&shared, &root, "pkg", Mode::Require).unwrap(),
        Resolved::File(pkg.join("lib/cjs.js"))
    );
    assert_eq!(
        resolve_specifier(&shared, &root, "pkg/sub", Mode::Require).unwrap(),
        Resolved::File(pkg.join("lib/sub.js"))
    );
    assert!(resolve_specifier(&shared, &root, "pkg/hidden", Mode::Require).is_err());
    assert_eq!(
        resolve_specifier(&shared, &root, "plain", Mode::Require).unwrap(),
        Resolved::File(plain.join("entry.js"))
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn esm_sniffing_and_file_urls() {
    assert!(looks_like_esm("import x from 'y';\n"));
    assert!(looks_like_esm("const a = 1;\nexport default a;\n"));
    assert!(looks_like_esm("export { a };"));
    assert!(!looks_like_esm("const m = await import('x');\n"));
    assert!(!looks_like_esm("module.exports = { exported: 1 };\n"));
    let path = Path::new("/tmp/with space/#x.js");
    assert_eq!(path_from_url_or_path(&file_url(path)), path);
}

#[test]
fn package_names_split_with_scopes() {
    assert_eq!(split_package("a"), ("a", ""));
    assert_eq!(split_package("a/b/c"), ("a", "b/c"));
    assert_eq!(split_package("@s/a"), ("@s/a", ""));
    assert_eq!(split_package("@s/a/b"), ("@s/a", "b"));
}
