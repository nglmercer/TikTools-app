//! Module system for QuickJS guests: ES modules, CommonJS, JSON, the
//! `tiktools:events` capability module and a deny-by-default
//! `node:module`.
//!
//! Everything a guest can reach is inside its own plugin directory: every
//! resolved path is canonicalised and must stay under the plugin root, so
//! neither `..` segments nor symlinks escape it. Native `.node` files are
//! never loadable by path; they only exist as the allowlisted package
//! aliases the host authorised before load (see `NativeAliases`).

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

// ---------------------------------------------------------------------
// Specifier resolution
// ---------------------------------------------------------------------

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

fn is_relative(spec: &str) -> bool {
    spec == "."
        || spec == ".."
        || spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with('/')
}

pub(crate) fn resolve_specifier(
    shared: &HostShared,
    from_dir: &Path,
    spec: &str,
    mode: Mode,
) -> Result<Resolved, String> {
    if spec == EVENTS_CAPABILITY {
        return if shared.events.is_some() {
            Ok(Resolved::Builtin(EVENTS_CAPABILITY))
        } else {
            Err(format!(
                "module `{EVENTS_CAPABILITY}` is not granted to this plugin"
            ))
        };
    }
    if spec == NODE_MODULE || spec == "module" {
        return Ok(Resolved::Builtin(NODE_MODULE));
    }
    if spec.starts_with("node:") {
        return Err(format!(
            "builtin module `{spec}` is not available to plugins"
        ));
    }
    if shared.aliases.contains_key(spec) {
        return Ok(Resolved::Alias(spec.to_owned()));
    }
    let candidate = if is_relative(spec) {
        let base = if spec.starts_with('/') {
            PathBuf::from(spec)
        } else {
            from_dir.join(spec)
        };
        resolve_path(&base)?
    } else {
        resolve_package(shared, from_dir, spec, mode)?
    };
    contained(shared, &candidate, spec).map(Resolved::File)
}

fn contained(shared: &HostShared, path: &Path, spec: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|_| format!("Cannot find module `{spec}` ({})", path.display()))?;
    if !canonical.starts_with(&shared.root) {
        return Err(format!("module `{spec}` escapes the plugin directory"));
    }
    Ok(canonical)
}

/// Resolves a path to a file: exact file, extension probing, then a
/// directory (`package.json` main, then `index`).
fn resolve_path(base: &Path) -> Result<PathBuf, String> {
    if base.is_file() {
        return Ok(base.to_owned());
    }
    for ext in EXTENSIONS {
        let mut probe = base.as_os_str().to_owned();
        probe.push(format!(".{ext}"));
        let probe = PathBuf::from(probe);
        if probe.is_file() {
            return Ok(probe);
        }
    }
    if base.is_dir() {
        return resolve_directory(base);
    }
    Err(format!("Cannot find module `{}`", base.display()))
}

fn resolve_directory(dir: &Path) -> Result<PathBuf, String> {
    if let Some(package) = read_package_json(&dir.join("package.json")) {
        if let Some(main) = package.get("main").and_then(serde_json::Value::as_str) {
            // `main` pointing at its own directory would recurse forever.
            let target = dir.join(main);
            if target != dir {
                if let Ok(found) = resolve_path(&target) {
                    return Ok(found);
                }
            }
        }
    }
    for ext in EXTENSIONS {
        let index = dir.join(format!("index.{ext}"));
        if index.is_file() {
            return Ok(index);
        }
    }
    Err(format!("Cannot find module `{}`", dir.display()))
}

fn read_package_json(path: &Path) -> Option<serde_json::Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn split_package(spec: &str) -> (&str, &str) {
    let name_end = if spec.starts_with('@') {
        spec.match_indices('/')
            .nth(1)
            .map_or(spec.len(), |(index, _)| index)
    } else {
        spec.find('/').unwrap_or(spec.len())
    };
    let (name, rest) = spec.split_at(name_end);
    (name, rest.trim_start_matches('/'))
}

fn resolve_package(
    shared: &HostShared,
    from_dir: &Path,
    spec: &str,
    mode: Mode,
) -> Result<PathBuf, String> {
    let (name, subpath) = split_package(spec);
    if name.is_empty() || name.contains("..") || name.contains('\\') {
        return Err(format!("invalid module specifier `{spec}`"));
    }
    for dir in from_dir.ancestors() {
        if !dir.starts_with(&shared.root) {
            break;
        }
        let package_dir = dir.join("node_modules").join(name);
        if !package_dir.is_dir() {
            continue;
        }
        return resolve_in_package(&package_dir, subpath, mode, spec);
    }
    Err(format!("Cannot find module `{spec}`"))
}

fn resolve_in_package(
    package_dir: &Path,
    subpath: &str,
    mode: Mode,
    spec: &str,
) -> Result<PathBuf, String> {
    let package = read_package_json(&package_dir.join("package.json"));
    if let Some(exports) = package.as_ref().and_then(|json| json.get("exports")) {
        let key = if subpath.is_empty() {
            ".".to_owned()
        } else {
            format!("./{subpath}")
        };
        let target = exports_target(exports, &key, mode)
            .ok_or_else(|| format!("module `{spec}` is not exported by its package"))?;
        return resolve_path(&package_dir.join(target.trim_start_matches("./")));
    }
    if !subpath.is_empty() {
        return resolve_path(&package_dir.join(subpath));
    }
    if let Some(package) = &package {
        let field = match mode {
            Mode::Import => package
                .get("module")
                .and_then(serde_json::Value::as_str)
                .or_else(|| package.get("main").and_then(serde_json::Value::as_str)),
            Mode::Require => package.get("main").and_then(serde_json::Value::as_str),
        };
        if let Some(main) = field {
            if let Ok(found) = resolve_path(&package_dir.join(main)) {
                return Ok(found);
            }
        }
    }
    resolve_directory(package_dir)
}

fn condition_matches(key: &str, mode: Mode) -> bool {
    match key {
        "default" | "node" => true,
        "import" | "module" | "module-sync" => mode == Mode::Import,
        "require" => mode == Mode::Require,
        _ => false,
    }
}

fn condition_target(value: &serde_json::Value, mode: Mode) -> Option<String> {
    match value {
        serde_json::Value::String(target) => Some(target.clone()),
        serde_json::Value::Array(items) => {
            items.iter().find_map(|item| condition_target(item, mode))
        }
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(key, _)| condition_matches(key, mode))
            .find_map(|(_, value)| condition_target(value, mode)),
        _ => None,
    }
}

fn exports_target(exports: &serde_json::Value, key: &str, mode: Mode) -> Option<String> {
    match exports {
        serde_json::Value::String(_) | serde_json::Value::Array(_) => (key == ".")
            .then(|| condition_target(exports, mode))
            .flatten(),
        serde_json::Value::Object(map) => {
            let has_subpaths = map.keys().any(|key| key.starts_with('.'));
            if has_subpaths {
                if let Some(value) = map.get(key) {
                    return condition_target(value, mode);
                }
                // `./features/*` style patterns.
                map.iter().find_map(|(pattern, value)| {
                    let (prefix, suffix) = pattern.split_once('*')?;
                    let middle = key.strip_prefix(prefix)?.strip_suffix(suffix)?;
                    condition_target(value, mode).map(|target| target.replace('*', middle))
                })
            } else if key == "." {
                condition_target(exports, mode)
            } else {
                None
            }
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Source classification
// ---------------------------------------------------------------------

fn nearest_package_type(shared: &HostShared, path: &Path) -> Option<String> {
    for dir in path.parent()?.ancestors() {
        if !dir.starts_with(&shared.root) {
            break;
        }
        if let Some(package) = read_package_json(&dir.join("package.json")) {
            return package
                .get("type")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        }
    }
    None
}

/// Lightweight ESM sniff: a top-level `import`/`export` statement or
/// `import.meta`. Dynamic `import(...)` alone does not make a file ESM.
fn looks_like_esm(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        if line.starts_with("import.meta") {
            return true;
        }
        if let Some(rest) = line.strip_prefix("import") {
            return rest.starts_with(|c: char| {
                c.is_whitespace() || c == '{' || c == '*' || c == '"' || c == '\''
            }) && !rest.trim_start().starts_with('(');
        }
        if let Some(rest) = line.strip_prefix("export") {
            return rest.starts_with(|c: char| c.is_whitespace() || c == '{' || c == '*');
        }
        line.contains("import.meta")
    })
}

fn is_esm_file(shared: &HostShared, path: &Path, source: &str) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mjs") => true,
        Some("cjs") => false,
        _ => match nearest_package_type(shared, path).as_deref() {
            Some("module") => true,
            Some("commonjs") => false,
            _ => looks_like_esm(source),
        },
    }
}

fn read_source(path: &Path) -> Result<String, String> {
    let length = fs::metadata(path)
        .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?
        .len();
    if length > MAX_SOURCE_BYTES {
        return Err(format!(
            "`{}` is larger than the {MAX_SOURCE_BYTES} byte source limit",
            path.display()
        ));
    }
    fs::read_to_string(path).map_err(|error| format!("cannot read `{}`: {error}", path.display()))
}

fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' | b':' => {
                url.push(byte as char);
            }
            other => url.push_str(&format!("%{other:02X}")),
        }
    }
    url
}

fn path_from_url_or_path(value: &str) -> PathBuf {
    let Some(rest) = value.strip_prefix("file://") else {
        return PathBuf::from(value);
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 3 <= bytes.len() {
            let decoded = std::str::from_utf8(&bytes[index + 1..index + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok());
            if let Some(byte) = decoded {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

fn base_dir_of(shared: &HostShared, base: &str) -> PathBuf {
    let path = Path::new(base);
    if path.is_absolute() {
        path.parent()
            .map_or_else(|| shared.root.clone(), Path::to_owned)
    } else {
        shared.root.clone()
    }
}

// ---------------------------------------------------------------------
// rquickjs resolver / loader
// ---------------------------------------------------------------------

pub(crate) struct PluginResolver(pub(crate) Rc<HostShared>);
pub(crate) struct PluginLoader(pub(crate) Rc<HostShared>);

impl Resolver for PluginResolver {
    fn resolve<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        let from = base_dir_of(&self.0, base);
        match resolve_specifier(&self.0, &from, name, Mode::Import) {
            Ok(Resolved::File(path)) => Ok(path.to_string_lossy().into_owned()),
            Ok(Resolved::Builtin(name)) => Ok(name.to_owned()),
            Ok(Resolved::Alias(alias)) => Ok(format!("{NATIVE_ALIAS_PREFIX}{alias}")),
            Err(message) => Err(Exception::throw_message(ctx, &message)),
        }
    }
}

impl Loader for PluginLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
        let shared = &self.0;
        match name {
            EVENTS_CAPABILITY => return Module::declare_def::<EventsModule, _>(ctx.clone(), name),
            NODE_MODULE => return Module::declare_def::<NodeModuleModule, _>(ctx.clone(), name),
            _ => {}
        }
        if let Some(alias) = name.strip_prefix(NATIVE_ALIAS_PREFIX) {
            let exports = load_alias(ctx, shared, alias)?;
            shared
                .wrapped
                .borrow_mut()
                .insert(name.to_owned(), Persistent::save(ctx, exports));
            return Module::declare_def::<WrappedModule, _>(ctx.clone(), name);
        }
        let path = PathBuf::from(name);
        declare_file(ctx, shared, &path, name)
    }
}

/// Declares the plugin's entry module (ES module or CommonJS).
pub(crate) fn declare_entry<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    entry: &Path,
) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
    declare_file(ctx, shared, entry, &entry.to_string_lossy())
}

fn declare_file<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    path: &Path,
    name: &str,
) -> rquickjs::Result<Module<'js, rquickjs::module::Declared>> {
    if !path.starts_with(&shared.root) {
        return Err(Exception::throw_message(
            ctx,
            &format!("module `{name}` escapes the plugin directory"),
        ));
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("node") => {
            return Err(Exception::throw_message(
                ctx,
                &format!("native addon `{name}` is not allowlisted"),
            ))
        }
        Some("json") => {
            let text =
                read_source(path).map_err(|message| Exception::throw_message(ctx, &message))?;
            ctx.json_parse(text.as_str())?; // validate before declaring
            return Module::declare(ctx.clone(), name, format!("export default {text};"));
        }
        _ => {}
    }
    let source = read_source(path).map_err(|message| Exception::throw_message(ctx, &message))?;
    if is_esm_file(shared, path, &source) {
        let module = Module::declare(ctx.clone(), name, source)?;
        let meta = module.meta()?;
        meta.set("url", file_url(path))?;
        meta.set("filename", path.to_string_lossy().into_owned())?;
        meta.set(
            "dirname",
            path.parent().map(|dir| dir.to_string_lossy().into_owned()),
        )?;
        Ok(module)
    } else {
        let exports = run_commonjs(ctx, shared, path, source)?;
        shared
            .wrapped
            .borrow_mut()
            .insert(name.to_owned(), Persistent::save(ctx, exports));
        Module::declare_def::<WrappedModule, _>(ctx.clone(), name)
    }
}

// ---------------------------------------------------------------------
// CommonJS
// ---------------------------------------------------------------------

fn run_commonjs<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    path: &Path,
    source: String,
) -> rquickjs::Result<Value<'js>> {
    if let Some(cached) = shared.modules.borrow().get(path).cloned() {
        return cached.restore(ctx)?.get("exports");
    }
    let module = Object::new(ctx.clone())?;
    let exports = Object::new(ctx.clone())?;
    module.set("exports", exports.clone())?;
    module.set("id", path.to_string_lossy().into_owned())?;
    module.set("filename", path.to_string_lossy().into_owned())?;
    // Registered before execution so require cycles see the partial exports.
    shared
        .modules
        .borrow_mut()
        .insert(path.to_owned(), Persistent::save(ctx, module.clone()));
    let dir = path
        .parent()
        .map_or_else(|| shared.root.clone(), Path::to_owned);
    let require = make_require(ctx, shared, dir.clone())?;
    let wrapper: Function = ctx.eval(format!(
        "(function (exports, require, module, __filename, __dirname) {{{source}\n}})"
    ))?;
    let result = wrapper.call::<_, Value>((
        exports,
        require,
        module.clone(),
        path.to_string_lossy().into_owned(),
        dir.to_string_lossy().into_owned(),
    ));
    if let Err(error) = result {
        shared.modules.borrow_mut().remove(path);
        return Err(error);
    }
    module.get("exports")
}

pub(crate) fn make_require<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    base_dir: PathBuf,
) -> rquickjs::Result<Function<'js>> {
    let shared = Rc::clone(shared);
    Function::new(ctx.clone(), move |ctx: Ctx<'js>, spec: String| {
        require_spec(&ctx, &shared, &base_dir, &spec)
    })
}

fn require_spec<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    base_dir: &Path,
    spec: &str,
) -> rquickjs::Result<Value<'js>> {
    match resolve_specifier(shared, base_dir, spec, Mode::Require) {
        Ok(Resolved::Builtin(EVENTS_CAPABILITY)) => {
            events_object(ctx, shared).map(Object::into_value)
        }
        Ok(Resolved::Builtin(_)) => node_module_object(ctx, shared).map(Object::into_value),
        Ok(Resolved::Alias(alias)) => load_alias(ctx, shared, &alias),
        Ok(Resolved::File(path)) => require_file(ctx, shared, &path, spec),
        Err(message) => Err(Exception::throw_message(ctx, &message)),
    }
}

fn require_file<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    path: &Path,
    spec: &str,
) -> rquickjs::Result<Value<'js>> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("node") => Err(Exception::throw_message(
            ctx,
            &format!("native addon `{spec}` is not allowlisted"),
        )),
        Some("json") => {
            let text =
                read_source(path).map_err(|message| Exception::throw_message(ctx, &message))?;
            ctx.json_parse(text.as_str())
        }
        _ => {
            let source =
                read_source(path).map_err(|message| Exception::throw_message(ctx, &message))?;
            if is_esm_file(shared, path, &source) {
                return Err(Exception::throw_message(
                    ctx,
                    &format!("require() of ES module `{spec}` is not supported; use import"),
                ));
            }
            run_commonjs(ctx, shared, path, source)
        }
    }
}

fn load_alias<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
    alias: &str,
) -> rquickjs::Result<Value<'js>> {
    let Some(path) = shared.aliases.get(alias) else {
        return Err(Exception::throw_message(
            ctx,
            &format!("native addon `{alias}` is not allowlisted"),
        ));
    };
    if let Some(loaded) = shared.addons.borrow().get(path).cloned() {
        return loaded.restore(ctx);
    }
    let exports = super::native::load_addon(ctx, shared, path)?;
    shared
        .addons
        .borrow_mut()
        .insert(path.clone(), Persistent::save(ctx, exports.clone()));
    Ok(exports)
}

// ---------------------------------------------------------------------
// Builtin modules
// ---------------------------------------------------------------------

fn events_object<'js>(ctx: &Ctx<'js>, shared: &Rc<HostShared>) -> rquickjs::Result<Object<'js>> {
    let Some(events) = shared.events.clone() else {
        return Err(Exception::throw_message(
            ctx,
            &format!("module `{EVENTS_CAPABILITY}` is not granted to this plugin"),
        ));
    };
    let object = Object::new(ctx.clone())?;
    let one = Arc::clone(&events);
    object.set(
        "emit",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, event: Opt<Value<'js>>| {
            let json = to_json(&ctx, event.0, "emit: event")?;
            one.push_one(&json)
                .map_err(|message| Exception::throw_message(&ctx, &message))
        })?,
    )?;
    object.set(
        "emitMany",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, batch: Opt<Value<'js>>| {
            let json = to_json(&ctx, batch.0, "emitMany: batch")?;
            events
                .push_many(&json)
                .map(|count| count as f64)
                .map_err(|message| Exception::throw_message(&ctx, &message))
        })?,
    )?;
    Ok(object)
}

fn to_json<'js>(
    ctx: &Ctx<'js>,
    value: Option<Value<'js>>,
    what: &str,
) -> rquickjs::Result<serde_json::Value> {
    let not_serializable = || Exception::throw_message(ctx, &format!("{what} is not serializable"));
    let Some(value) = value else {
        return Err(not_serializable());
    };
    let text = ctx
        .json_stringify(value)
        .map_err(|_| not_serializable())?
        .ok_or_else(not_serializable)?
        .to_string()?;
    serde_json::from_str(&text).map_err(|_| not_serializable())
}

fn node_module_object<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
) -> rquickjs::Result<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    let shared = Rc::clone(shared);
    object.set(
        "createRequire",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, from: String| {
            let path = path_from_url_or_path(&from);
            let dir = if path.is_dir() {
                path
            } else {
                path.parent()
                    .map_or_else(|| shared.root.clone(), Path::to_owned)
            };
            if !dir.starts_with(&shared.root) {
                return Err(Exception::throw_message(
                    &ctx,
                    "createRequire base is outside the plugin directory",
                ));
            }
            make_require(&ctx, &shared, dir)
        })?,
    )?;
    Ok(object)
}

struct EventsModule;

impl ModuleDef for EventsModule {
    fn declare<'js>(decl: &Declarations<'js>) -> rquickjs::Result<()> {
        decl.declare("emit")?;
        decl.declare("emitMany")?;
        decl.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let shared = shared_of(ctx)?;
        let object = events_object(ctx, &shared)?;
        exports.export("emit", object.get::<_, Value>("emit")?)?;
        exports.export("emitMany", object.get::<_, Value>("emitMany")?)?;
        exports.export("default", object)?;
        Ok(())
    }
}

struct NodeModuleModule;

impl ModuleDef for NodeModuleModule {
    fn declare<'js>(decl: &Declarations<'js>) -> rquickjs::Result<()> {
        decl.declare("createRequire")?;
        decl.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let shared = shared_of(ctx)?;
        let object = node_module_object(ctx, &shared)?;
        exports.export("createRequire", object.get::<_, Value>("createRequire")?)?;
        exports.export("default", object)?;
        Ok(())
    }
}

fn is_identifier(name: &str) -> bool {
    const RESERVED: [&str; 14] = [
        "default", "class", "function", "var", "let", "const", "new", "delete", "import", "export",
        "return", "if", "else", "this",
    ];
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
        && !RESERVED.contains(&name)
}

/// ES view of an already-evaluated CommonJS module or native addon:
/// `default` is `module.exports` and every identifier-named own key is
/// also a named export, like Node's CJS interop.
struct WrappedModule;

fn wrapped_value<'js>(ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Value<'js>> {
    let shared = shared_of(ctx)?;
    let persisted = shared.wrapped.borrow().get(name).cloned();
    persisted
        .ok_or_else(|| Exception::throw_internal(ctx, "wrapped module state is missing"))?
        .restore(ctx)
}

fn wrapped_keys(value: &Value<'_>) -> Vec<String> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    object
        .keys::<String>()
        .filter_map(Result::ok)
        .filter(|key| is_identifier(key))
        .collect()
}

impl ModuleDef for WrappedModule {
    fn declare<'js>(decl: &Declarations<'js>) -> rquickjs::Result<()> {
        let module = decl.module();
        let ctx = module.ctx().clone();
        let name: String = module.name()?;
        let value = wrapped_value(&ctx, &name)?;
        decl.declare("default")?;
        for key in wrapped_keys(&value) {
            decl.declare(key)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let name: String = exports.module().name()?;
        let value = wrapped_value(ctx, &name)?;
        if let Some(object) = value.as_object() {
            for key in wrapped_keys(&value) {
                exports.export(key.as_str(), object.get::<_, Value>(key.as_str())?)?;
            }
        }
        exports.export("default", value)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
}
