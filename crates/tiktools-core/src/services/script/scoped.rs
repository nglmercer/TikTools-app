//! Separately bounded delegated script evaluation. Returned JSON is data;
//! every intent still requires the saved parent approval and its own sink grant.
use super::*;
use std::{cell::RefCell, sync::Arc};

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopedScriptLimits {
    pub source_bytes: usize,
    pub result_bytes: usize,
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_intents: usize,
    pub max_log_entries: usize,
    pub max_log_bytes: usize,
    pub max_log_entry_bytes: usize,
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    pub wall_clock_ms: u64,
    pub max_interrupts: u32,
    pub max_pending_jobs: usize,
    pub max_concurrent_evaluations: usize,
}

/// The private approval policy serializes these same limits into its exact
/// binding. Changing an evaluator limit therefore requires fresh approval.
pub(crate) const SCOPED_SCRIPT_LIMITS: ScopedScriptLimits = ScopedScriptLimits {
    source_bytes: MAX_SOURCE_BYTES,
    result_bytes: 256 * 1024,
    max_depth: 32,
    max_nodes: 8192,
    max_intents: 64,
    max_log_entries: 40,
    max_log_bytes: 64 * 1024,
    max_log_entry_bytes: 4096,
    memory_bytes: MEMORY_LIMIT_BYTES,
    stack_bytes: STACK_LIMIT_BYTES,
    wall_clock_ms: 5000,
    max_interrupts: INTERRUPT_BUDGET,
    max_pending_jobs: 2000,
    max_concurrent_evaluations: 4,
};

const AUTHORITY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const SCOPED_CANCELLED: &str = "scoped script authorization expired or evaluation was cancelled";
const SCOPED_BUDGET: &str = "scoped script execution budget exceeded";
const SCOPED_FAILED: &str = "scoped script evaluation failed";

struct Budget {
    started: Instant,
    last_authority_check: Cell<Instant>,
    ticks: Cell<u32>,
    exhausted: Cell<bool>,
    revoked: Cell<bool>,
    cancelled: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl Budget {
    fn check(&self, force_authority: bool) -> Result<(), String> {
        if self.revoked.get() {
            return Err(SCOPED_CANCELLED.into());
        }
        if self.exhausted.get()
            || self.started.elapsed() > Duration::from_millis(SCOPED_SCRIPT_LIMITS.wall_clock_ms)
        {
            self.exhausted.set(true);
            return Err(SCOPED_BUDGET.into());
        }
        let now = Instant::now();
        if force_authority
            || now.duration_since(self.last_authority_check.get()) >= AUTHORITY_POLL_INTERVAL
        {
            self.last_authority_check.set(now);
            if (self.cancelled)() {
                self.revoked.set(true);
                return Err(SCOPED_CANCELLED.into());
            }
        }
        if self.started.elapsed() > Duration::from_millis(SCOPED_SCRIPT_LIMITS.wall_clock_ms) {
            self.exhausted.set(true);
            return Err(SCOPED_BUDGET.into());
        }
        Ok(())
    }

    fn interrupt(&self) -> bool {
        self.ticks.set(self.ticks.get().saturating_add(1));
        if self.ticks.get() > SCOPED_SCRIPT_LIMITS.max_interrupts {
            self.exhausted.set(true);
        }
        self.check(false).is_err()
    }

    fn failure(&self) -> String {
        if self.revoked.get() {
            SCOPED_CANCELLED.into()
        } else if self.exhausted.get() {
            SCOPED_BUDGET.into()
        } else {
            SCOPED_FAILED.into()
        }
    }
}

#[derive(Default)]
struct ConsoleLogs {
    entries: Vec<String>,
    bytes: usize,
}

impl ScriptService {
    /// No host lock may be held while evaluating. `cancelled` is trusted host
    /// code and must synchronously revalidate saved/current authority without
    /// reentering this runtime. The caller owns the blocking-task lifetime.
    pub(crate) fn evaluate_scoped(
        &self,
        source: &str,
        event: &Value,
        cancelled: impl Fn() -> bool + Send + Sync + 'static,
    ) -> Result<Value, String> {
        let started = Instant::now();
        let budget = Rc::new(Budget {
            started,
            last_authority_check: Cell::new(started),
            ticks: Cell::new(0),
            exhausted: Cell::new(false),
            revoked: Cell::new(false),
            cancelled: Arc::new(cancelled),
        });
        budget.check(true)?;
        let program = format!("{}()", function_source(source)?);
        // Defense in depth: even a trusted caller cannot accidentally expose
        // Owner enrichment, settings or authority-looking JSON to this VM.
        let event = crate::services::automation_approval_policy::project_event(event);
        let event_json = serde_json::to_string(&event).map_err(|_| SCOPED_FAILED.to_owned())?;
        if event_json.len() > SCOPED_SCRIPT_LIMITS.result_bytes {
            return Err(SCOPED_BUDGET.into());
        }
        budget.check(true)?;
        let (runtime, context) = new_context().map_err(|_| SCOPED_FAILED.to_owned())?;
        let interrupt = Rc::clone(&budget);
        runtime.set_interrupt_handler(Some(Box::new(move || interrupt.interrupt())));
        let console = Rc::new(RefCell::new(ConsoleLogs::default()));
        let outcome = context.with(|ctx| -> Result<Value, String> {
            let run = || -> Result<Value, rquickjs::Error> {
                let event_value: JsValue = ctx.json_parse(event_json.as_str())?;
                let data = event_value
                    .as_object()
                    .map(|object| object.get::<_, Option<JsValue>>("data"))
                    .transpose()?
                    .flatten();
                let globals = ctx.globals();
                globals.set("event", event_value)?;
                globals.set("inputs", Object::new(ctx.clone())?)?;
                globals.set("globals", Object::new(ctx.clone())?)?;
                globals.set(
                    "data",
                    data.unwrap_or_else(|| JsValue::new_null(ctx.clone())),
                )?;
                install_scoped_console(&ctx, Rc::clone(&console), Rc::clone(&budget))?;
                let value: JsValue = ctx.eval(program.as_str())?;
                if value.is_promise() {
                    return Err(rquickjs::Exception::throw_type(
                        &ctx,
                        "scoped scripts must return synchronous JSON",
                    ));
                }
                let mut jobs = 0_usize;
                loop {
                    budget
                        .check(false)
                        .map_err(|_| rquickjs::Exception::throw_message(&ctx, SCOPED_CANCELLED))?;
                    if !ctx.execute_pending_job() {
                        break;
                    }
                    jobs += 1;
                    if jobs > SCOPED_SCRIPT_LIMITS.max_pending_jobs || budget.check(false).is_err()
                    {
                        budget.exhausted.set(!budget.revoked.get());
                        return Err(rquickjs::Exception::throw_message(&ctx, SCOPED_BUDGET));
                    }
                }
                budget
                    .check(true)
                    .map_err(|_| rquickjs::Exception::throw_message(&ctx, SCOPED_CANCELLED))?;
                let Some(rendered) = ctx.json_stringify(value)? else {
                    return Ok(Value::Null);
                };
                let rendered = rendered.to_cstring()?;
                if rendered.len() > SCOPED_SCRIPT_LIMITS.result_bytes {
                    budget.exhausted.set(true);
                    return Err(rquickjs::Exception::throw_message(&ctx, SCOPED_BUDGET));
                }
                serde_json::from_slice(rendered.as_ref())
                    .map_err(|_| rquickjs::Exception::throw_message(&ctx, SCOPED_FAILED))
            };
            run().map_err(|_| budget.failure())
        });
        drop(context);
        drop(runtime);
        budget.check(true)?;
        let value = finish_result(outcome?, &console.borrow().entries)?;
        budget.check(true)?;
        Ok(value)
    }
}

fn install_scoped_console<'js>(
    ctx: &Ctx<'js>,
    logs: Rc<RefCell<ConsoleLogs>>,
    budget: Rc<Budget>,
) -> Result<(), rquickjs::Error> {
    let console = Object::new(ctx.clone())?;
    for name in ["log", "info", "warn", "error", "debug"] {
        let logs = Rc::clone(&logs);
        let budget = Rc::clone(&budget);
        console.set(
            name,
            Function::new(
                ctx.clone(),
                move |callback_context: Ctx<'js>,
                      message: rquickjs::function::Opt<JsValue<'js>>|
                      -> Result<(), rquickjs::Error> {
                    budget.check(false).map_err(|_| {
                        rquickjs::Exception::throw_message(&callback_context, SCOPED_CANCELLED)
                    })?;
                    let mut logs = logs.borrow_mut();
                    if logs.entries.len() >= SCOPED_SCRIPT_LIMITS.max_log_entries
                        || logs.bytes >= SCOPED_SCRIPT_LIMITS.max_log_bytes
                    {
                        return Ok(());
                    }
                    // Only the first scalar argument is captured. Objects never invoke
                    // user toString/toJSON, and no guest-controlled value reaches tracing.
                    let text = match message.0 {
                        Some(value) if value.is_string() => {
                            let text = value
                                .as_string()
                                .expect("string kind checked")
                                .clone()
                                .to_cstring()?;
                            bounded_prefix(text.as_str(), SCOPED_SCRIPT_LIMITS.max_log_entry_bytes)
                                .to_owned()
                        }
                        Some(value) if value.is_null() => "null".into(),
                        Some(value) if value.is_undefined() => "undefined".into(),
                        Some(value) if value.is_bool() => {
                            value.as_bool().unwrap_or(false).to_string()
                        }
                        Some(value) if value.is_number() => {
                            value.as_number().unwrap_or(0.0).to_string()
                        }
                        Some(_) => "[object]".into(),
                        None => String::new(),
                    };
                    let available = SCOPED_SCRIPT_LIMITS.max_log_bytes - logs.bytes;
                    let text = bounded_prefix(&text, available).to_owned();
                    logs.bytes += text.len();
                    logs.entries.push(text);
                    budget.check(false).map_err(|_| {
                        rquickjs::Exception::throw_message(&callback_context, SCOPED_CANCELLED)
                    })
                },
            )?,
        )?;
    }
    ctx.globals().set("console", console)
}

fn bounded_prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn validate_structure(value: &Value) -> Result<(), String> {
    let mut pending = vec![(value, 0_usize)];
    let mut nodes = 0_usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if depth > SCOPED_SCRIPT_LIMITS.max_depth || nodes > SCOPED_SCRIPT_LIMITS.max_nodes {
            return Err("scoped script result exceeds its structural budget".into());
        }
        match value {
            Value::Array(values) => pending.extend(values.iter().map(|value| (value, depth + 1))),
            Value::Object(values) => {
                pending.extend(values.values().map(|value| (value, depth + 1)))
            }
            _ => {}
        }
    }
    Ok(())
}

fn finish_result(mut value: Value, captured: &[String]) -> Result<Value, String> {
    validate_structure(&value)?;
    let mut intents = 0_usize;
    for key in ["emit", tiktools_plugin_api::AUDIO_PLAY_INTENT, "fetch"] {
        if let Some(value) = value.get(key).filter(|value| !value.is_null()) {
            intents = intents.saturating_add(value.as_array().map_or(1, Vec::len));
        }
    }
    if intents > SCOPED_SCRIPT_LIMITS.max_intents {
        return Err("scoped script intent count exceeds its budget".into());
    }
    let mut logs = Vec::new();
    let mut log_bytes = 0_usize;
    if let Some(returned) = value.get("log").filter(|value| !value.is_null()) {
        let entries = match returned {
            Value::Array(values) => values.iter().collect::<Vec<_>>(),
            value => vec![value],
        };
        for entry in entries {
            let entry = entry.as_str().ok_or("scoped script logs must be strings")?;
            log_bytes = log_bytes.saturating_add(entry.len());
            if entry.len() > SCOPED_SCRIPT_LIMITS.max_log_entry_bytes
                || logs.len() >= SCOPED_SCRIPT_LIMITS.max_log_entries
                || log_bytes > SCOPED_SCRIPT_LIMITS.max_log_bytes
            {
                return Err("scoped script logs exceed their budget".into());
            }
            logs.push(entry.to_owned());
        }
    }
    for captured in captured {
        if logs.len() >= SCOPED_SCRIPT_LIMITS.max_log_entries
            || log_bytes >= SCOPED_SCRIPT_LIMITS.max_log_bytes
        {
            break;
        }
        let text =
            bounded_prefix(captured, SCOPED_SCRIPT_LIMITS.max_log_bytes - log_bytes).to_owned();
        log_bytes += text.len();
        logs.push(text);
    }
    if !logs.is_empty() {
        if !value.is_object() {
            value = serde_json::json!({"value":value});
        }
        value
            .as_object_mut()
            .expect("object projection created")
            .insert("log".into(), serde_json::json!(logs));
    }
    // Captured console entries count toward the final structural budget too.
    validate_structure(&value)?;
    if serde_json::to_vec(&value)
        .map_err(|_| SCOPED_FAILED.to_owned())?
        .len()
        > SCOPED_SCRIPT_LIMITS.result_bytes
    {
        return Err("scoped script result exceeds its byte budget".into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
