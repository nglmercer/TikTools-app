//! The QuickJS plugin host: one runtime, one context, one guest plugin.
//! Owned and driven by a single thread (see the module docs).

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Instant,
};

use rquickjs::{
    function::{Rest, This},
    CatchResultExt, Context, Ctx, Function, Object, Persistent, Runtime, Value,
};

use super::{
    describe_error,
    events::EventsPush,
    modules::{HostShared, NativeAliases, PluginLoader, PluginResolver, SharedHandle},
    timers::{self, SharedTimers},
};

/// Interrupt-handler ticks (roughly 10k bytecode ops each) one guest call
/// may use. A spinning guest then terminates with a `RangeError` instead of
/// wedging the owner thread; each call and each pump starts a fresh budget.
const CALL_TICK_BUDGET: u32 = 400_000;
const MEMORY_LIMIT_BYTES: usize = 512 * 1024 * 1024;
const STACK_LIMIT_BYTES: usize = 2 * 1024 * 1024;
pub(super) const BUDGET_MESSAGE: &str = "RangeError: Maximum loop iterations exceeded";

pub(crate) struct HostConfig {
    pub name: String,
    pub version: String,
    pub directory: PathBuf,
    pub entry: String,
    pub aliases: NativeAliases,
    pub events: Option<Arc<EventsPush>>,
    pub wake: super::runtime::WakeNotifier,
}

pub(crate) struct QuickJsHost {
    // Field order is drop order: every retained JavaScript value must go
    // before the runtime that owns it.
    plugin: Option<Persistent<Object<'static>>>,
    shared: Rc<HostShared>,
    timers: SharedTimers,
    context: Context,
    runtime: Runtime,
    ticks: Arc<AtomicU32>,
    name: String,
    version: String,
}

impl QuickJsHost {
    pub(crate) fn load(config: HostConfig) -> Result<Self, String> {
        let HostConfig {
            name,
            version,
            directory,
            entry,
            aliases,
            events,
            wake,
        } = config;
        let root = std::fs::canonicalize(&directory)
            .map_err(|error| format!("plugin directory is not usable: {error}"))?;
        let entry_path = std::fs::canonicalize(root.join(&entry))
            .map_err(|_| format!("entry `{entry}` was not found"))?;
        if !entry_path.starts_with(&root) {
            return Err(format!("entry `{entry}` escapes the plugin directory"));
        }

        let runtime = Runtime::new().map_err(|error| error.to_string())?;
        runtime.set_memory_limit(MEMORY_LIMIT_BYTES);
        runtime.set_max_stack_size(STACK_LIMIT_BYTES);
        let ticks = Arc::new(AtomicU32::new(0));
        {
            let ticks = Arc::clone(&ticks);
            runtime.set_interrupt_handler(Some(Box::new(move || {
                ticks.fetch_add(1, Ordering::Relaxed) >= CALL_TICK_BUDGET
            })));
        }
        let shared = HostShared::new(name.clone(), root, aliases, events, wake);
        runtime.set_loader(PluginResolver(Rc::clone(&shared)), PluginLoader(Rc::clone(&shared)));
        let context = Context::full(&runtime).map_err(|error| error.to_string())?;
        let timers: SharedTimers = Rc::default();

        let mut host = Self {
            plugin: None,
            shared: Rc::clone(&shared),
            timers: Rc::clone(&timers),
            context,
            runtime,
            ticks,
            name,
            version,
        };
        if let Err(error) = host.start(&entry_path) {
            host.teardown();
            return Err(error);
        }
        Ok(host)
    }

    fn start(&mut self, entry_path: &Path) -> Result<(), String> {
        let shared = Rc::clone(&self.shared);
        let timers = Rc::clone(&self.timers);
        let name = self.name.clone();
        let version = self.version.clone();
        let entry = entry_path.to_owned();
        self.reset_budget();
        let plugin = self.context.with(|ctx| -> Result<_, String> {
            ctx.store_userdata(SharedHandle(Rc::clone(&shared)))
                .map_err(|_| "plugin state could not be stored".to_owned())?;
            install_globals(&ctx, &timers, &name).map_err(|error| describe_error(&ctx, error))?;
            let module =
                super::modules::declare_entry(&ctx, &shared, &entry).map_err(|e| describe_error(&ctx, e))?;
            let (module, promise) = module.eval().map_err(|e| describe_error(&ctx, e))?;
            settle(&ctx, &timers, &shared, &name, promise.into_value())?;
            let namespace = module.namespace().map_err(|e| describe_error(&ctx, e))?;
            let default: Value = namespace
                .get("default")
                .map_err(|e| describe_error(&ctx, e))?;
            let plugin = match default.into_object() {
                Some(object) => object,
                None => namespace,
            };
            let hook_context = hook_context(&ctx, &name, &version, None)?;
            call_hook(&ctx, &timers, &shared, &name, &plugin, "onLoad", hook_context)?;
            Ok(Persistent::save(&ctx, plugin))
        })?;
        self.plugin = Some(plugin);
        Ok(())
    }

    fn reset_budget(&self) {
        self.ticks.store(0, Ordering::Relaxed);
    }

    /// Whether the last guest call ran out of its execution budget.
    fn budget_exhausted(&self) -> bool {
        self.ticks.load(Ordering::Relaxed) > CALL_TICK_BUDGET
    }

    /// Runs one `PluginCall` through the guest `call(request, context)`
    /// export; Promise results are awaited (timers and microtasks drain
    /// inside the call).
    pub(crate) fn call(
        &mut self,
        request: &serde_json::Value,
        call_context: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let plugin = self
            .plugin
            .clone()
            .ok_or_else(|| format!("plugin `{}` is not loaded", self.name))?;
        let timers = Rc::clone(&self.timers);
        let shared = Rc::clone(&self.shared);
        let name = self.name.clone();
        self.reset_budget();
        let outcome = self.context.with(|ctx| -> Result<serde_json::Value, String> {
            let plugin = plugin.restore(&ctx).map_err(|e| describe_error(&ctx, e))?;
            let function: Value = plugin
                .get("call")
                .map_err(|e| describe_error(&ctx, e))?;
            let Some(function) = function.into_function() else {
                return Err("plugin must export call(request, context)".to_owned());
            };
            let request = ctx
                .json_parse(request.to_string())
                .map_err(|e| describe_error(&ctx, e))?;
            let call_context = ctx
                .json_parse(call_context.to_string())
                .map_err(|e| describe_error(&ctx, e))?;
            let result: Value = function
                .call((This(plugin), request, call_context))
                .map_err(|e| describe_error(&ctx, e))?;
            let result = settle(&ctx, &timers, &shared, &name, result)?;
            let text = ctx
                .json_stringify(result)
                .map_err(|e| describe_error(&ctx, e))?
                .ok_or_else(|| "plugin call returned no result".to_owned())?
                .to_string()
                .map_err(|e| describe_error(&ctx, e))?;
            serde_json::from_str(&text).map_err(|error| format!("call result is not JSON: {error}"))
        });
        match outcome {
            Err(_) if self.budget_exhausted() => Err(BUDGET_MESSAGE.to_owned()),
            other => other,
        }
    }

    /// One non-blocking event-loop turn: due timers, then microtasks.
    pub(crate) fn pump(&mut self) {
        let timers = Rc::clone(&self.timers);
        let shared = Rc::clone(&self.shared);
        let name = self.name.clone();
        self.reset_budget();
        self.context.with(|ctx| {
            super::native::pump(&shared);
            while ctx.execute_pending_job() {}
            timers::run_due(&ctx, &timers, &name);
            while ctx.execute_pending_job() {}
        });
    }

    /// Runs guest `onUnload`, then releases the runtime.
    pub(crate) fn unload(&mut self) -> Result<(), String> {
        let result = match self.plugin.clone() {
            None => Ok(()),
            Some(plugin) => {
                let timers = Rc::clone(&self.timers);
                let shared = Rc::clone(&self.shared);
                let name = self.name.clone();
                let version = self.version.clone();
                self.reset_budget();
                self.context.with(|ctx| -> Result<(), String> {
                    let plugin = plugin.restore(&ctx).map_err(|e| describe_error(&ctx, e))?;
                    let hook_context = hook_context(&ctx, &name, &version, Some("unload"))?;
                    call_hook(&ctx, &timers, &shared, &name, &plugin, "onUnload", hook_context)
                })
            }
        };
        self.teardown();
        result
    }

    /// Releases every retained JavaScript value, then collects.
    fn teardown(&mut self) {
        self.plugin = None;
        self.timers.borrow_mut().clear();
        self.shared.clear();
        super::native::shutdown(&self.shared);
        self.runtime.run_gc();
        super::native::pump(&self.shared);
    }
}

impl Drop for QuickJsHost {
    fn drop(&mut self) {
        self.teardown();
    }
}

fn hook_context<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    version: &str,
    reason: Option<&str>,
) -> Result<Object<'js>, String> {
    let build = || -> rquickjs::Result<Object<'js>> {
        let object = Object::new(ctx.clone())?;
        object.set("name", name)?;
        object.set("version", version)?;
        if let Some(reason) = reason {
            object.set("reason", reason)?;
        }
        Ok(object)
    };
    build().map_err(|e| describe_error(ctx, e))
}

/// Calls an optional lifecycle hook and awaits its result.
fn call_hook<'js>(
    ctx: &Ctx<'js>,
    timers: &SharedTimers,
    shared: &Rc<HostShared>,
    plugin_name: &str,
    plugin: &Object<'js>,
    hook: &str,
    hook_context: Object<'js>,
) -> Result<(), String> {
    let function: Value = plugin.get(hook).map_err(|e| describe_error(ctx, e))?;
    let Some(function) = function.into_function() else {
        return Ok(());
    };
    let result: Value = function
        .call((This(plugin.clone()), hook_context))
        .catch(ctx)
        .map_err(|error| error.to_string())?;
    settle(ctx, timers, shared, plugin_name, result).map(|_| ())
}

/// Drives microtasks and timers until `value` is settled. Non-promises
/// return immediately.
fn settle<'js>(
    ctx: &Ctx<'js>,
    timers: &SharedTimers,
    shared: &Rc<HostShared>,
    plugin: &str,
    value: Value<'js>,
) -> Result<Value<'js>, String> {
    let Some(promise) = value.as_promise().cloned() else {
        while ctx.execute_pending_job() {}
        return Ok(value);
    };
    loop {
        super::native::pump(shared);
        while ctx.execute_pending_job() {}
        if let Some(result) = promise.result::<Value>() {
            return result.map_err(|error| describe_error(ctx, error));
        }
        let next = timers::run_due(ctx, timers, plugin);
        if promise.state() != rquickjs::promise::PromiseState::Pending {
            continue;
        }
        match next {
            Some(due) => {
                let now = Instant::now();
                if due > now {
                    super::native::wait_for_event(shared, due - now);
                }
            }
            None => {
                if super::native::has_pending_work(shared) {
                    super::native::wait_for_event(shared, std::time::Duration::from_millis(100));
                } else {
                    return Err(
                        "promise never settled: no timers or native work can resolve it".to_owned(),
                    );
                }
            }
        }
    }
}

fn install_globals<'js>(
    ctx: &Ctx<'js>,
    timers: &SharedTimers,
    plugin: &str,
) -> rquickjs::Result<()> {
    timers::install(ctx, timers)?;
    let console = Object::new(ctx.clone())?;
    for (method, level) in [
        ("log", tracing::Level::INFO),
        ("info", tracing::Level::INFO),
        ("warn", tracing::Level::WARN),
        ("error", tracing::Level::ERROR),
        ("debug", tracing::Level::DEBUG),
    ] {
        let plugin = plugin.to_owned();
        console.set(
            method,
            Function::new(ctx.clone(), move |parts: Rest<Value<'js>>| {
                let text = parts
                    .0
                    .iter()
                    .map(display_value)
                    .collect::<Vec<_>>()
                    .join(" ");
                match level {
                    tracing::Level::ERROR => tracing::error!(plugin = %plugin, "guest: {text}"),
                    tracing::Level::WARN => tracing::warn!(plugin = %plugin, "guest: {text}"),
                    tracing::Level::DEBUG => tracing::debug!(plugin = %plugin, "guest: {text}"),
                    _ => tracing::info!(plugin = %plugin, "guest: {text}"),
                }
            })?,
        )?;
    }
    ctx.globals().set("console", console)?;
    ctx.eval::<(), _>(PRELUDE)?;
    Ok(())
}

fn display_value(value: &Value<'_>) -> String {
    if let Some(text) = value.as_string() {
        return text.to_string().unwrap_or_default();
    }
    let ctx = value.ctx();
    ctx.json_stringify(value.clone())
        .ok()
        .flatten()
        .and_then(|text| text.to_string().ok())
        .unwrap_or_else(|| format!("{value:?}"))
}

/// Small, dependency-free globals (see `prelude.js`).
const PRELUDE: &str = include_str!("prelude.js");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guests_get_text_codecs_timers_and_console() {
        let dir = std::env::temp_dir().join(format!("tiktools-host-globals-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.js"),
            r#"export default {
  async call() {
    const text = "héllo ✓ 😀";
    const bytes = new TextEncoder().encode(text);
    const back = new TextDecoder().decode(bytes);
    const late = await new Promise((resolve) => setTimeout(() => resolve("timer"), 5));
    console.log("hello from guest");
    return { bytes: bytes.length, same: back === text, late, bad: new TextDecoder().decode(new Uint8Array([0xff, 0x41])) };
  },
};
"#,
        )
        .unwrap();
        let mut host = QuickJsHost::load(HostConfig {
            name: "globals".to_owned(),
            version: "1.0.0".to_owned(),
            directory: dir.clone(),
            entry: "index.js".to_owned(),
            aliases: Default::default(),
            events: None,
            wake: Arc::new(|| {}),
        })
        .unwrap();
        let result = host
            .call(&serde_json::json!({"type": "poll"}), &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            result,
            serde_json::json!({"bytes": 15, "same": true, "late": "timer", "bad": "\u{fffd}A"})
        );
        host.unload().unwrap();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn guests_get_buffer_and_url() {
        let dir = std::env::temp_dir().join(format!("tiktools-host-url-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.js"),
            r#"export default {
  call() {
    const u = new URL("https://User:pw@Example.com:8443/a/b/../c?x=1&y=two%20words#frag");
    u.searchParams.append("z", "a b");
    const rel = new URL("../d?q=1", "https://example.com/a/b/c").href;
    const buf = Buffer.from("héllo", "utf8");
    return {
      host: u.host, origin: u.origin, path: u.pathname, y: u.searchParams.get("y"),
      search: u.search, hash: u.hash, user: u.username, rel,
      hex: buf.toString("hex"), b64: buf.toString("base64"),
      round: Buffer.from(buf.toString("base64"), "base64").toString(),
      concat: Buffer.concat([Buffer.from("ab"), Buffer.from("cd")]).toString(),
      isBuffer: Buffer.isBuffer(buf), defaultPort: new URL("http://a.com:80/").port === "",
    };
  },
};
"#,
        )
        .unwrap();
        let mut host = QuickJsHost::load(HostConfig {
            name: "url".to_owned(),
            version: "1.0.0".to_owned(),
            directory: dir.clone(),
            entry: "index.js".to_owned(),
            aliases: Default::default(),
            events: None,
            wake: Arc::new(|| {}),
        })
        .unwrap();
        let result = host
            .call(&serde_json::json!({"type": "poll"}), &serde_json::json!({}))
            .unwrap();
        assert_eq!(
            result,
            serde_json::json!({
                "host": "example.com:8443", "origin": "https://example.com:8443",
                "path": "/a/c", "y": "two words", "search": "?x=1&y=two+words&z=a+b",
                "hash": "#frag", "user": "User", "rel": "https://example.com/a/d?q=1",
                "hex": "68c3a96c6c6f", "b64": "aMOpbGxv", "round": "héllo",
                "concat": "abcd", "isBuffer": true, "defaultPort": true
            })
        );
        host.unload().unwrap();
        std::fs::remove_dir_all(dir).ok();
    }
}
