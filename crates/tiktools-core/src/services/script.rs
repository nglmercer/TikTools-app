//! Bounded JavaScript execution for automation scripts.
//!
//! A small host surface around QuickJS (`rquickjs`). It exposes JSON values
//! only; there is no Node API, filesystem, network, process, or WebView
//! handle available to guest code. Every evaluation gets a fresh runtime with
//! a memory cap, a stack cap and an execution budget enforced through
//! QuickJS' interrupt handler.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use rquickjs::{Context, Ctx, Function, Object, Runtime, Value as JsValue};
use serde_json::Value;

const MAX_SOURCE_BYTES: usize = 128 * 1024;
const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
/// QuickJS calls the interrupt handler roughly every 10k bytecode ops, so
/// this bounds a script to about 20M ops; the wall clock backs it up for
/// slow host calls.
const INTERRUPT_BUDGET: u32 = 2_000;
const WALL_CLOCK_BUDGET: Duration = Duration::from_secs(5);
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const STACK_LIMIT_BYTES: usize = 512 * 1024;
const BUDGET_MESSAGE: &str = "Maximum loop iterations exceeded";

#[derive(Default)]
pub struct ScriptService;

impl ScriptService {
    pub fn validate(&self, source: &str) -> Result<(), String> {
        let function = function_source(source)?;
        let (runtime, context) = new_context()?;
        let tripped = Rc::new(Cell::new(false));
        install_budget(&runtime, Rc::clone(&tripped));
        let outcome = context.with(|ctx| {
            ctx.eval::<JsValue, _>(function).map(|_| ()).map_err(|error| {
                if tripped.get() {
                    BUDGET_MESSAGE.to_owned()
                } else {
                    describe_error(&ctx, error)
                }
            })
        });
        drop(context);
        drop(runtime);
        outcome
    }

    pub fn evaluate(&self, source: &str, event: &Value, inputs: &Value) -> Result<Value, String> {
        let program = format!("JSON.stringify({}())", function_source(source)?);
        let event_json = serde_json::to_string(event).map_err(|error| error.to_string())?;
        let inputs_json = serde_json::to_string(inputs).map_err(|error| error.to_string())?;
        let (runtime, context) = new_context()?;
        let tripped = Rc::new(Cell::new(false));
        install_budget(&runtime, Rc::clone(&tripped));
        let outcome = context.with(|ctx| {
            let run = || -> Result<Option<String>, rquickjs::Error> {
                let globals = ctx.globals();
                let event_value = ctx.json_parse(event_json.as_str())?;
                let inputs_value = ctx.json_parse(inputs_json.as_str())?;
                // `data` is the event payload shorthand used by the
                // automation contract. `inputs` stays a separate
                // host-provided object.
                let data = match event_value.as_object() {
                    Some(object) => object.get::<_, Option<JsValue>>("data")?,
                    None => None,
                };
                globals.set("event", event_value)?;
                globals.set("inputs", inputs_value)?;
                match data {
                    Some(data) if !data.is_undefined() => globals.set("data", data)?,
                    _ => globals.set("data", JsValue::new_null(ctx.clone()))?,
                }
                install_console(&ctx)?;
                let result: JsValue = ctx.eval(program.as_str())?;
                // Settle microtasks scheduled by the script before reading
                // the result, like a host event loop would.
                while ctx.execute_pending_job() {}
                if result.is_undefined() {
                    Ok(None)
                } else {
                    Ok(Some(result.get::<String>()?))
                }
            };
            run().map_err(|error| {
                if tripped.get() {
                    BUDGET_MESSAGE.to_owned()
                } else {
                    describe_error(&ctx, error)
                }
            })
        });
        drop(context);
        drop(runtime);
        json_result(outcome?)
    }
}

fn new_context() -> Result<(Runtime, Context), String> {
    let runtime = Runtime::new().map_err(|error| error.to_string())?;
    runtime.set_memory_limit(MEMORY_LIMIT_BYTES);
    runtime.set_max_stack_size(STACK_LIMIT_BYTES);
    // Base intrinsics only plus JSON/Promise/etc: no timers, no I/O.
    let context = Context::full(&runtime).map_err(|error| error.to_string())?;
    Ok((runtime, context))
}

fn install_budget(runtime: &Runtime, tripped: Rc<Cell<bool>>) {
    let started = Instant::now();
    let mut ticks = 0_u32;
    runtime.set_interrupt_handler(Some(Box::new(move || {
        ticks += 1;
        let exhausted = ticks > INTERRUPT_BUDGET || started.elapsed() > WALL_CLOCK_BUDGET;
        if exhausted {
            tripped.set(true);
        }
        exhausted
    })));
}

fn install_console(ctx: &Ctx<'_>) -> Result<(), rquickjs::Error> {
    let console = Object::new(ctx.clone())?;
    for name in ["log", "info", "warn", "error", "debug"] {
        console.set(
            name,
            Function::new(ctx.clone(), |message: rquickjs::function::Rest<String>| {
                tracing::debug!(target: "automation_script", "{}", message.0.join(" "));
            })?,
        )?;
    }
    ctx.globals().set("console", console)
}

fn describe_error(ctx: &Ctx<'_>, error: rquickjs::Error) -> String {
    if matches!(error, rquickjs::Error::Exception) {
        let caught = ctx.catch();
        if let Some(object) = caught.as_object() {
            let name = object.get::<_, Option<String>>("name").ok().flatten();
            let message = object.get::<_, Option<String>>("message").ok().flatten();
            if let (Some(name), Some(message)) = (name, message) {
                return format!("{name}: {message}");
            }
        }
        return caught
            .as_string()
            .and_then(|text| text.to_string().ok())
            .unwrap_or_else(|| "script threw an exception".to_owned());
    }
    error.to_string()
}

fn function_source(source: &str) -> Result<String, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("automation script is larger than the 128 KiB limit".to_owned());
    }
    // A function body gives scripts a useful `return` statement while keeping
    // declarations local to one invocation. The result is serialized inside
    // the VM, so no Rust object or host callback crosses the boundary.
    Ok(format!(
        "(function () {{\nfunction log(value) {{ console.log(value); }}\n{source}\n}})"
    ))
}

fn json_result(rendered: Option<String>) -> Result<Value, String> {
    let Some(rendered) = rendered else {
        return Ok(Value::Null);
    };
    if rendered.len() > MAX_JSON_BYTES {
        return Err("automation script result is larger than the 2 MiB limit".to_owned());
    }
    serde_json::from_str(&rendered)
        .map_err(|error| format!("automation script returned invalid JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn evaluates_json_with_event_and_inputs() {
        let result = ScriptService
            .evaluate(
                "return { user: event.user.uniqueId, value: inputs.amount + 1, comment: data.comment };",
                &json!({"user": {"uniqueId": "viewer"}, "data": {"comment": "hello"}}),
                &json!({"amount": 4}),
            )
            .unwrap();
        assert_eq!(
            result,
            json!({"user": "viewer", "value": 5, "comment": "hello"})
        );
    }

    #[test]
    fn rejects_infinite_loops() {
        let error = ScriptService
            .evaluate("while (true) {}", &Value::Null, &Value::Null)
            .unwrap_err();
        assert!(error.contains("Maximum loop iterations"), "{error}");
    }

    #[test]
    fn reports_syntax_errors_without_running_partial_code() {
        let error = ScriptService.validate("return {").unwrap_err();
        assert!(error.contains("SyntaxError"), "{error}");
    }

    #[test]
    fn validation_bounds_code_that_escapes_the_wrapper() {
        let error = ScriptService
            .validate("}); while (true) {} (function () {")
            .unwrap_err();
        assert_eq!(error, BUDGET_MESSAGE);
    }

    #[test]
    fn undefined_results_become_null() {
        let result = ScriptService
            .evaluate("var x = 1;", &Value::Null, &Value::Null)
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn guests_have_no_host_apis() {
        let result = ScriptService
            .evaluate(
                "return [typeof require, typeof process, typeof fetch, typeof setTimeout, typeof std, typeof os];",
                &Value::Null,
                &Value::Null,
            )
            .unwrap();
        assert_eq!(
            result,
            json!(["undefined", "undefined", "undefined", "undefined", "undefined", "undefined"])
        );
    }

    #[test]
    fn missing_event_data_is_null_and_log_is_available() {
        let result = ScriptService
            .evaluate("log('hi'); return data;", &json!({"user": 1}), &Value::Null)
            .unwrap();
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn runaway_memory_is_rejected() {
        let error = ScriptService
            .evaluate(
                "var s = 'x'; while (true) { s = s + s; }",
                &Value::Null,
                &Value::Null,
            )
            .unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn oversized_source_and_runtime_errors_are_reported() {
        assert!(ScriptService
            .validate(&"a".repeat(MAX_SOURCE_BYTES + 1))
            .is_err());
        let error = ScriptService
            .evaluate("throw new TypeError('boom');", &Value::Null, &Value::Null)
            .unwrap_err();
        assert!(error.contains("TypeError: boom"), "{error}");
    }
}
