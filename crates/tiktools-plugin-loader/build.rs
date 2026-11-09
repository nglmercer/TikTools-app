//! Compiles the Node-API symbol shim (`native/node_api_shim.c`).
//!
//! Native addons (napi-rs `.node` files) import `napi_*` symbols from the
//! process. The shim exports them and forwards each call to a function
//! table the host installs at runtime. It is embedded in the loader and
//! materialised into a private temp directory on first use.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=native/node_api_shim.c");
    println!("cargo::rerun-if-env-changed=CC");
    println!("cargo::rustc-check-cfg=cfg(node_api_host)");

    if std::env::var_os("CARGO_FEATURE_NAPI_VM_NODE_API").is_none() {
        return;
    }
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if !matches!(target_os.as_str(), "linux" | "macos" | "windows") {
        return;
    }
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let library_name = match target_os.as_str() {
        "macos" => "libtiktools_node_api_shim.dylib",
        // Windows addon import libraries name node.exe as their provider.
        "windows" => "node.exe",
        _ => "libtiktools_node_api_shim.so",
    };
    let library = out_dir.join(library_name);
    let target = std::env::var("TARGET").unwrap_or_default();
    let host = std::env::var("HOST").unwrap_or_default();
    let target_specific_cc = std::env::var_os(format!("CC_{target}"))
        .or_else(|| std::env::var_os(format!("CC_{}", target.replace('-', "_"))));
    let compiler = target_specific_cc.unwrap_or_else(|| {
        if target_os == "windows" && target_env == "gnu" && target != host {
            format!("{target_arch}-w64-mingw32-gcc").into()
        } else if target_os == "windows" && target_env == "msvc" {
            std::env::var_os("CC").unwrap_or_else(|| "cl.exe".into())
        } else if target_os == "windows" && target_env == "gnu" {
            std::env::var_os("CC").unwrap_or_else(|| "gcc".into())
        } else {
            std::env::var_os("CC").unwrap_or_else(|| "cc".into())
        }
    });
    let compiler_name = compiler.to_string_lossy().into_owned();
    let mut command = std::process::Command::new(compiler);
    if target_os == "windows" && target_env == "msvc" {
        command
            .args(["/nologo", "/O2", "/LD", "/TC"])
            .arg("native/node_api_shim.c")
            .arg(format!(
                "/Fo{}",
                out_dir.join("node_api_shim.obj").display()
            ))
            .arg("/link")
            .arg(format!("/IMPLIB:{}", out_dir.join("node.lib").display()))
            .arg(format!("/OUT:{}", library.display()));
    } else {
        let link_flag = if target_os == "macos" {
            "-dynamiclib"
        } else {
            "-shared"
        };
        command.args(["-std=c11", "-O2", "-fvisibility=hidden", link_flag]);
        if target_os != "windows" {
            command.arg("-fPIC");
        } else {
            command.arg(format!(
                "-Wl,--out-implib,{}",
                out_dir.join("libnode.exe.a").display()
            ));
        }
        command
            .arg("native/node_api_shim.c")
            .arg("-o")
            .arg(&library);
    }
    let status = command.status().unwrap_or_else(|error| {
        panic!("the Node-API host needs a C compiler: could not run '{compiler_name}': {error}")
    });
    assert!(
        status.success(),
        "failed to compile the Node-API symbol shim"
    );
    println!(
        "cargo::rustc-env=TIKTOOLS_NODE_API_SHIM_PATH={}",
        library.display()
    );
    println!("cargo::rustc-cfg=node_api_host");
}
