//! Deny-by-default guest specifiers and QuickJS resolver integration.
use super::*;

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
