//! CommonJS execution, partial-export cycles, require and allowlisted native aliases.
use super::*;

pub(super) fn run_commonjs<'js>(
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

pub(super) fn load_alias<'js>(
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
    let exports = crate::quickjs::native::load_addon(ctx, shared, path)?;
    shared
        .addons
        .borrow_mut()
        .insert(path.clone(), Persistent::save(ctx, exports.clone()));
    Ok(exports)
}
