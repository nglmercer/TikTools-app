//! Native (Node-API) addon support for QuickJS guests.

#[cfg(node_api_host)]
mod imp {
    pub(crate) use super::super::napi::{has_pending_work, load_addon, pump, shutdown, wait_for_event, NativeSlot};
}

#[cfg(not(node_api_host))]
mod imp {
    use std::{cell::RefCell, path::Path, rc::Rc, time::Duration};

    use rquickjs::{Ctx, Exception, Value};

    use super::super::modules::HostShared;

    pub(crate) type NativeSlot = RefCell<Option<()>>;

    pub(crate) fn load_addon<'js>(
        ctx: &Ctx<'js>,
        _shared: &Rc<HostShared>,
        path: &Path,
    ) -> rquickjs::Result<Value<'js>> {
        Err(Exception::throw_message(
            ctx,
            &format!("native addons are not supported by this build ({})", path.display()),
        ))
    }
    pub(crate) fn pump(_: &HostShared) {}
    pub(crate) fn has_pending_work(_: &HostShared) -> bool {
        false
    }
    pub(crate) fn wait_for_event(_: &HostShared, _: Duration) -> bool {
        false
    }
    pub(crate) fn shutdown(_: &HostShared) {}
}

pub(super) use imp::*;
