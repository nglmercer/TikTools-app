//! Module system for QuickJS guests: ES modules, CommonJS, JSON, the
//! `tiktools:events` capability module and a deny-by-default
//! `node:module`.
//!
//! Everything a guest can reach is inside its own plugin directory: every
//! resolved path is canonicalised and must stay under the plugin root, so
//! neither `..` segments nor symlinks escape it. Native `.node` files are
//! never loadable by path; they only exist as the allowlisted package
//! aliases the host authorised before load (see `NativeAliases`).

mod builtins;
mod commonjs;
mod loading;
mod packages;
mod paths;
mod resolution;
mod source;
mod wrappers;

use builtins::{events_object, node_module_object, EventsModule, NodeModuleModule};
pub(crate) use commonjs::make_require;
use commonjs::{load_alias, run_commonjs};
pub(crate) use loading::declare_entry;
use packages::resolve_package;
#[cfg(test)]
use packages::split_package;
use paths::{
    base_dir_of, contained, file_url, path_from_url_or_path, read_package_json, resolve_directory,
    resolve_path,
};
pub(crate) use resolution::resolve_specifier;
#[cfg(test)]
use source::looks_like_esm;
use source::{is_esm_file, read_source};
use wrappers::WrappedModule;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use rquickjs::{
    function::Opt,
    loader::{ImportAttributes, Loader, Resolver},
    module::{Declarations, Exports, ModuleDef},
    Ctx, Exception, Function, JsLifetime, Module, Object, Persistent, Value,
};

use super::events::{EventsPush, EVENTS_CAPABILITY};

/// Largest guest source file the host will read.
const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

const NODE_MODULE: &str = "node:module";
const NATIVE_ALIAS_PREFIX: &str = "native-alias:";

/// Authorised native addons: package alias to canonical `.node` path.
pub(crate) type NativeAliases = HashMap<String, PathBuf>;

/// Per-plugin state shared by the resolver, the loader and every
/// native function handed to the guest. Lives on the owner thread only.
pub(crate) struct HostShared {
    pub(crate) plugin: String,
    /// Canonical plugin directory: the only tree guests can read from.
    pub(crate) root: PathBuf,
    pub(crate) aliases: NativeAliases,
    pub(crate) events: Option<Arc<EventsPush>>,
    /// CommonJS `module` objects by canonical path.
    modules: RefCell<HashMap<PathBuf, Persistent<Object<'static>>>>,
    /// Pre-evaluated `module.exports` for synthetic ES wrappers, by module name.
    wrapped: RefCell<HashMap<String, Persistent<Value<'static>>>>,
    /// Loaded native addons by canonical path.
    pub(crate) addons: RefCell<HashMap<PathBuf, Persistent<Value<'static>>>>,
    pub(crate) wake: super::runtime::WakeNotifier,
    pub(crate) native: super::native::NativeSlot,
}

impl HostShared {
    pub(crate) fn new(
        plugin: String,
        root: PathBuf,
        aliases: NativeAliases,
        events: Option<Arc<EventsPush>>,
        wake: super::runtime::WakeNotifier,
    ) -> Rc<Self> {
        Rc::new(Self {
            plugin,
            root,
            aliases,
            events,
            modules: RefCell::default(),
            wrapped: RefCell::default(),
            addons: RefCell::default(),
            wake,
            native: RefCell::default(),
        })
    }

    /// Drops every retained JavaScript value. Must run before the runtime
    /// is freed.
    pub(crate) fn clear(&self) {
        self.modules.borrow_mut().clear();
        self.wrapped.borrow_mut().clear();
        self.addons.borrow_mut().clear();
    }
}

/// Userdata handle so `ModuleDef`s (which are plain functions) can reach
/// the plugin's shared state.
#[derive(Clone)]
pub(crate) struct SharedHandle(pub(crate) Rc<HostShared>);

// SAFETY: contains no JavaScript lifetimes; every retained value is stored
// as `Persistent` and restored through a context of the same runtime.
unsafe impl<'js> JsLifetime<'js> for SharedHandle {
    type Changed<'to> = SharedHandle;
}

fn shared_of(ctx: &Ctx<'_>) -> rquickjs::Result<Rc<HostShared>> {
    ctx.userdata::<SharedHandle>()
        .map(|handle| Rc::clone(&handle.0))
        .ok_or_else(|| Exception::throw_internal(ctx, "plugin host state is missing"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Import,
    Require,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resolved {
    File(PathBuf),
    Builtin(&'static str),
    Alias(String),
}

const EXTENSIONS: [&str; 4] = ["js", "mjs", "cjs", "json"];

pub(crate) struct PluginResolver(pub(crate) Rc<HostShared>);
pub(crate) struct PluginLoader(pub(crate) Rc<HostShared>);

#[cfg(test)]
mod tests;
