//! Process-wide Node-API symbol shim: a tiny C library exporting every
//! `napi_*` symbol and forwarding it to [`TABLE`]. It is loaded with global
//! visibility before any addon so the addon's imports resolve against it.

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
};

#[cfg(unix)]
use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
#[cfg(target_os = "windows")]
use libloading::os::windows::{
    Library, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
    LOAD_LIBRARY_SEARCH_USER_DIRS,
};

use super::table::{ApiTable, TABLE};

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn AddDllDirectory(new_directory: *const u16) -> *mut std::ffi::c_void;
}

static SHIM: OnceLock<Mutex<Option<Arc<NodeApiShim>>>> = OnceLock::new();
/// Addon libraries stay mapped for the process lifetime: napi-rs addons
/// may retain imports and threads beyond a single plugin generation.
static ADDONS: OnceLock<Mutex<HashMap<PathBuf, Arc<Library>>>> = OnceLock::new();

pub(super) struct NodeApiShim {
    _library: Library,
    #[cfg(target_os = "windows")]
    _dynamic_lookup: Library,
}

impl NodeApiShim {
    pub(super) fn load() -> Result<Arc<Self>, String> {
        let slot = SHIM.get_or_init(|| Mutex::new(None));
        let mut slot = slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(shim) = slot.as_ref() {
            return Ok(Arc::clone(shim));
        }
        let shim = Arc::new(Self::load_uncached()?);
        *slot = Some(Arc::clone(&shim));
        Ok(shim)
    }

    fn load_uncached() -> Result<Self, String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let bytes = include_bytes!(env!("TIKTOOLS_NODE_API_SHIM_PATH"));
        let root = loop {
            let candidate = std::env::temp_dir().join(format!(
                "tiktools-node-api-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("cannot create private Node-API shim directory: {error}"))
                }
            }
        };
        #[cfg(target_os = "linux")]
        let path = root.join("libtiktools_node_api_shim.so");
        #[cfg(target_os = "macos")]
        let path = root.join("libtiktools_node_api_shim.dylib");
        #[cfg(target_os = "windows")]
        let path = root.join("node.exe");
        let written = (|| {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o500))?;
            }
            Ok::<_, std::io::Error>(())
        })();
        if let Err(error) = written {
            let _ = fs::remove_dir_all(&root);
            return Err(format!("cannot materialize Node-API symbol shim: {error}"));
        }

        // MSVC napi-rs addons resolve Node-API at registration time from the
        // already-loaded `node` module, which matches `node.dll` but not
        // `node.exe`; map the same image under both names.
        #[cfg(target_os = "windows")]
        let dynamic_path = {
            let dll = root.join("node.dll");
            if let Err(error) = fs::copy(&path, &dll) {
                let _ = fs::remove_dir_all(&root);
                return Err(format!("cannot materialize Node-API dynamic-lookup shim: {error}"));
            }
            use std::os::windows::ffi::OsStrExt;
            let mut directory: Vec<u16> = root.as_os_str().encode_wide().collect();
            directory.push(0);
            if unsafe { AddDllDirectory(directory.as_ptr()) }.is_null() {
                let error = std::io::Error::last_os_error();
                let _ = fs::remove_dir_all(&root);
                return Err(format!("cannot register private Node-API shim directory: {error}"));
            }
            dll
        };

        #[cfg(unix)]
        let library = unsafe { Library::open(Some(path.as_os_str()), RTLD_NOW | RTLD_GLOBAL) };
        #[cfg(target_os = "windows")]
        let library = unsafe { Library::new(path.as_os_str()) };
        let library = library.map_err(|error| {
            let _ = fs::remove_dir_all(&root);
            format!("cannot load Node-API symbol shim: {error}")
        })?;
        install(&library).inspect_err(|_| {
            let _ = fs::remove_dir_all(&root);
        })?;
        #[cfg(target_os = "windows")]
        let dynamic_lookup = {
            let second = unsafe { Library::new(dynamic_path.as_os_str()) }
                .map_err(|error| format!("cannot load Node-API dynamic-lookup shim: {error}"))?;
            install(&second)?;
            second
        };
        // Unix lets a mapped shared object be unlinked; do not leave temp
        // copies behind. Windows keeps the files while mapped.
        #[cfg(unix)]
        let _ = fs::remove_dir_all(&root);
        Ok(Self {
            _library: library,
            #[cfg(target_os = "windows")]
            _dynamic_lookup: dynamic_lookup,
        })
    }

    /// Opens (once per path) an addon library. Libraries are never unloaded.
    pub(super) fn open_addon(&self, filename: &Path) -> Result<Arc<Library>, String> {
        let cache = ADDONS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(library) = cache.get(filename) {
            return Ok(Arc::clone(library));
        }
        #[cfg(unix)]
        let library = unsafe { Library::open(Some(filename.as_os_str()), RTLD_NOW) };
        #[cfg(target_os = "windows")]
        let library = unsafe {
            Library::load_with_flags(
                filename.as_os_str(),
                LOAD_LIBRARY_SEARCH_DEFAULT_DIRS
                    | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR
                    | LOAD_LIBRARY_SEARCH_USER_DIRS,
            )
        };
        let library = Arc::new(library.map_err(|error| error.to_string())?);
        cache.insert(filename.to_owned(), Arc::clone(&library));
        Ok(library)
    }
}

fn install(library: &Library) -> Result<(), String> {
    let install: unsafe extern "C" fn(*const ApiTable) =
        match unsafe { library.get(b"tiktools_install_node_api_table\0") } {
            Ok(symbol) => *symbol,
            Err(error) => return Err(format!("invalid Node-API symbol shim: {error}")),
        };
    unsafe { install(&TABLE) };
    Ok(())
}
