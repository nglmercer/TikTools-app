//! Capability-bound events and the restricted node:module interface.
use super::*;

pub(super) struct EventsModule;

pub(super) struct NodeModuleModule;

pub(super) fn events_object<'js>(
    ctx: &Ctx<'js>,
    shared: &Rc<HostShared>,
) -> rquickjs::Result<Object<'js>> {
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

pub(super) fn node_module_object<'js>(
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
