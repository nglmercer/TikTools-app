//! Synthetic ES exports for already-evaluated CommonJS and native values.
use super::*;

/// ES view of an already-evaluated CommonJS module or native addon:
/// `default` is `module.exports` and every identifier-named own key is
/// also a named export, like Node's CJS interop.
pub(super) struct WrappedModule;

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
