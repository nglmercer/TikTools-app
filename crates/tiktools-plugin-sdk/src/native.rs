//! Native ABI adapter used by `tiktools_export_native_plugin!`.

use super::*;
use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    panic::{catch_unwind, AssertUnwindSafe},
    slice,
};
use tiktools_plugin_api::{PluginBuffer, PluginStatus, MAX_FRAME_BYTES};

struct NativePluginState<P: Plugin> {
    plugin: P,
    context: PluginContext,
    initialization_error: Option<String>,
}

pub fn create<P>() -> *mut c_void
where
    P: Plugin + Default,
{
    let context = PluginContext::for_native_abi_v1();
    let mut plugin = P::default();
    let initialization_error = plugin
        .initialize(&context)
        .err()
        .map(|error| error.to_string());
    Box::into_raw(Box::new(NativePluginState {
        plugin,
        context,
        initialization_error,
    })) as *mut c_void
}

/// # Safety
///
/// `context` must be a pointer returned by `create::<P>` that has not
/// already been destroyed.
pub unsafe fn destroy<P>(context: *mut c_void)
where
    P: Plugin,
{
    if context.is_null() {
        return;
    }
    // SAFETY: the pointer was allocated by `create::<P>` and is consumed
    // exactly once by the native ABI destroy function.
    let mut state = unsafe { Box::from_raw(context.cast::<NativePluginState<P>>()) };
    let _ = state.plugin.shutdown(&state.context);
}

/// # Safety
///
/// `context` must come from `create::<P>`, `request_ptr` must refer to a
/// readable request buffer for `request_len` bytes, and `response` must
/// point to writable storage owned by the ABI caller.
pub unsafe fn handle_message<P>(
    context: *mut c_void,
    request_ptr: *const u8,
    request_len: usize,
    response: *mut PluginBuffer,
) -> PluginStatus
where
    P: Plugin,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        if context.is_null() || response.is_null() {
            return Err(PluginStatus::InvalidRequest);
        }
        if request_ptr.is_null() || request_len > MAX_FRAME_BYTES {
            return Err(PluginStatus::InvalidRequest);
        }
        // SAFETY: the host supplies the request pointer and length for the
        // duration of this call; validation above bounds the slice.
        let request = unsafe { slice::from_raw_parts(request_ptr, request_len) };
        // SAFETY: the pointer was allocated by `create::<P>`.
        let state = unsafe { &mut *context.cast::<NativePluginState<P>>() };
        if state.initialization_error.is_some() {
            return Err(PluginStatus::InternalError);
        }
        // Native ABI v1 receives the raw typed call. The process
        // adapter has an outer PluginRequest envelope, but adding
        // that envelope here would break existing native plugins.
        let call = serde_json::from_slice::<PluginCall>(request)
            .map_err(|_| PluginStatus::InvalidRequest)?;
        let result =
            dispatch_plugin_call(&mut state.plugin, &state.context, call).map_err(|error| {
                match error {
                    PluginError::InvalidRequest(_) | PluginError::UnsupportedAction(_) => {
                        PluginStatus::InvalidRequest
                    }
                    PluginError::CapabilityUnavailable(_) | PluginError::Other(_) => {
                        PluginStatus::InternalError
                    }
                }
            })?;
        let bytes = serde_json::to_vec(&result).map_err(|_| PluginStatus::InternalError)?;
        write_buffer(response, bytes);
        Ok(())
    }));
    match result {
        Ok(Ok(())) => PluginStatus::Ok,
        Ok(Err(status)) => status,
        Err(_) => PluginStatus::InternalError,
    }
}

/// # Safety
///
/// `response` must point to a buffer previously initialized by this
/// adapter, or to writable `PluginBuffer` storage.
pub unsafe fn free_buffer(response: *mut PluginBuffer) {
    if response.is_null() {
        return;
    }
    // SAFETY: the host passes the same buffer that this adapter allocated.
    let buffer = unsafe { &mut *response };
    if !buffer.ptr.is_null() && buffer.capacity >= buffer.len {
        // SAFETY: pointer, length, and capacity came from `write_buffer`.
        unsafe { drop(Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity)) };
    }
    *buffer = PluginBuffer::empty();
}

fn write_buffer(response: *mut PluginBuffer, bytes: Vec<u8>) {
    // SAFETY: the caller validated that `response` is non-null.
    let response = unsafe { &mut *response };
    if bytes.is_empty() {
        *response = PluginBuffer::empty();
        return;
    }
    let mut bytes = ManuallyDrop::new(bytes);
    *response = PluginBuffer {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
        capacity: bytes.capacity(),
    };
}
