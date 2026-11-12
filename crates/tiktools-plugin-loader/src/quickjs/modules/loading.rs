//! QuickJS module declarations and synthetic interop selection.
use super::*;

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
