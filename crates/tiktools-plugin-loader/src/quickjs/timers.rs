//! `setTimeout`/`setInterval` for guests.
//!
//! The owner thread sleeps with no timeout while idle, so timers only fire
//! while a call (or a host-event pump) is draining the event loop: a guest
//! timer always runs inside the call that is waiting for it, exactly like
//! the previous runtime. Timer state never leaves the owner thread.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};

use rquickjs::{function::Opt, function::Rest, Ctx, Function, Persistent, Value};

/// Pending timers per plugin: bounds guest memory use.
const MAX_TIMERS: usize = 4096;

struct Timer {
    due: Instant,
    interval: Option<Duration>,
    callback: Persistent<Function<'static>>,
    args: Vec<Persistent<Value<'static>>>,
}

#[derive(Default)]
pub(super) struct Timers {
    next_id: u32,
    entries: BTreeMap<u32, Timer>,
}

pub(super) type SharedTimers = Rc<RefCell<Timers>>;

impl Timers {
    pub(super) fn next_due(&self) -> Option<Instant> {
        self.entries.values().map(|timer| timer.due).min()
    }

    /// Drops every pending timer. Must run before the JS runtime is freed:
    /// the persistent values hold references into it.
    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}

fn delay_duration(delay: Option<f64>) -> Duration {
    let millis = delay.unwrap_or(0.0);
    if millis.is_finite() && millis > 0.0 {
        Duration::from_millis(millis.min(u32::MAX as f64) as u64)
    } else {
        Duration::ZERO
    }
}

fn schedule<'js>(
    ctx: &Ctx<'js>,
    timers: &SharedTimers,
    callback: Function<'js>,
    delay: Option<f64>,
    args: Vec<Value<'js>>,
    repeat: bool,
) -> rquickjs::Result<f64> {
    let mut state = timers.borrow_mut();
    if state.entries.len() >= MAX_TIMERS {
        return Err(rquickjs::Exception::throw_range(
            ctx,
            "too many pending timers",
        ));
    }
    state.next_id = state.next_id.wrapping_add(1).max(1);
    let id = state.next_id;
    let delay = delay_duration(delay);
    state.entries.insert(
        id,
        Timer {
            due: Instant::now() + delay,
            interval: repeat.then_some(delay.max(Duration::from_millis(1))),
            callback: Persistent::save(ctx, callback),
            args: args
                .into_iter()
                .map(|value| Persistent::save(ctx, value))
                .collect(),
        },
    );
    Ok(f64::from(id))
}

pub(super) fn install<'js>(ctx: &Ctx<'js>, timers: &SharedTimers) -> rquickjs::Result<()> {
    let globals = ctx.globals();
    for (name, repeat) in [("setTimeout", false), ("setInterval", true)] {
        let timers = Rc::clone(timers);
        globals.set(
            name,
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>,
                      callback: Function<'js>,
                      delay: Opt<f64>,
                      args: Rest<Value<'js>>| {
                    schedule(&ctx, &timers, callback, delay.0, args.0, repeat)
                },
            )?,
        )?;
    }
    {
        let timers = Rc::clone(timers);
        globals.set(
            "setImmediate",
            Function::new(
                ctx.clone(),
                move |ctx: Ctx<'js>, callback: Function<'js>, args: Rest<Value<'js>>| {
                    schedule(&ctx, &timers, callback, Some(0.0), args.0, false)
                },
            )?,
        )?;
    }
    for name in ["clearTimeout", "clearInterval", "clearImmediate"] {
        let timers = Rc::clone(timers);
        globals.set(
            name,
            Function::new(ctx.clone(), move |id: Opt<f64>| {
                if let Some(id) = id.0 {
                    if id >= 0.0 && id <= f64::from(u32::MAX) {
                        timers.borrow_mut().entries.remove(&(id as u32));
                    }
                }
            })?,
        )?;
    }
    Ok(())
}

/// Runs every timer that is due now, draining microtasks after each
/// callback. Returns the next pending deadline. Callback failures are
/// logged: one throwing timer must not kill the plugin.
pub(super) fn run_due<'js>(
    ctx: &Ctx<'js>,
    timers: &SharedTimers,
    plugin: &str,
) -> Option<Instant> {
    let now = Instant::now();
    let mut due: Vec<(Instant, u32)> = timers
        .borrow()
        .entries
        .iter()
        .filter(|(_, timer)| timer.due <= now)
        .map(|(id, timer)| (timer.due, *id))
        .collect();
    due.sort();
    for (_, id) in due {
        let fired = {
            let mut state = timers.borrow_mut();
            let Some(timer) = state.entries.get_mut(&id) else {
                continue; // cleared by an earlier callback in this pass
            };
            let callback = timer.callback.clone();
            let args = timer.args.clone();
            match timer.interval {
                Some(interval) => timer.due = Instant::now() + interval,
                None => {
                    state.entries.remove(&id);
                }
            }
            (callback, args)
        };
        let (callback, args) = fired;
        let outcome = (|| -> rquickjs::Result<()> {
            let callback = callback.restore(ctx)?;
            let args = args
                .into_iter()
                .map(|value| value.restore(ctx))
                .collect::<rquickjs::Result<Vec<_>>>()?;
            callback.call::<_, Value>((Rest(args),))?;
            Ok(())
        })();
        if let Err(error) = outcome {
            tracing::warn!(
                plugin = %plugin,
                error = %super::describe_error(ctx, error),
                "guest timer callback failed"
            );
        }
        while ctx.execute_pending_job() {}
    }
    timers.borrow().next_due()
}
