//! Trusted native dynamic-library runtime.

use std::path::Path;

use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};

use crate::{PluginInstance, PluginLoaderError, PluginRuntime};

#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
use libloading::Library;
#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
use std::ffi::c_void;
#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
use tiktools_plugin_api::{
    abi::{PluginInit, PluginStatus, TIKTOOLS_PLUGIN_INIT_SYMBOL},
    TikToolsPluginApi,
};

#[derive(Default)]
pub struct NativePluginRuntime;

impl PluginRuntime for NativePluginRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Native
    }

    #[cfg(any(not(feature = "native-plugins"), target_os = "android"))]
    fn load(
        &self,
        _manifest: &PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Err(PluginLoaderError::RuntimeUnavailable(
            "native plugins unavailable: this build/platform has no validated native ABI backend"
                .to_owned(),
        ))
    }

    #[cfg(all(feature = "native-plugins", not(target_os = "android")))]
    fn load(
        &self,
        manifest: &PluginManifest,
        directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        if manifest.runtime != PluginRuntimeKind::Native {
            return Err(PluginLoaderError::Runtime(
                "runtime kind mismatch".to_owned(),
            ));
        }
        let entry = manifest.entry.as_str();
        let path = directory.join(entry);
        let library = unsafe { Library::new(&path) }
            .map_err(|error| PluginLoaderError::Runtime(format!("{}: {error}", path.display())))?;
        let init: PluginInit = unsafe {
            *library.get(TIKTOOLS_PLUGIN_INIT_SYMBOL).map_err(|error| {
                PluginLoaderError::Runtime(format!("missing plugin init symbol: {error}"))
            })?
        };
        let api = unsafe { init() };
        if api.is_null() {
            return Err(PluginLoaderError::Runtime(
                "plugin returned a null API table".to_owned(),
            ));
        }
        let api_ref = unsafe { &*api };
        if !api_ref.is_compatible() {
            return Err(PluginLoaderError::Runtime(format!(
                "plugin ABI/protocol mismatch (abi={}, protocol={})",
                api_ref.abi_version, api_ref.protocol_version
            )));
        }
        let context = api_ref
            .create
            .map(|create| unsafe { create() })
            .unwrap_or(std::ptr::null_mut());
        Ok(Box::new(NativePluginInstance {
            id: manifest.id.clone(),
            _library: library,
            api,
            context,
        }))
    }
}

#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
struct NativePluginInstance {
    id: String,
    // The library must outlive the API table and any plugin context.
    _library: Library,
    api: *const TikToolsPluginApi,
    context: *mut c_void,
}

#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
// Native plugin code is trusted by definition. The host serializes calls and
// keeps the library alive for the whole instance lifetime.
unsafe impl Send for NativePluginInstance {}

#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
impl PluginInstance for NativePluginInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        let api = unsafe { &*self.api };
        let handle_message = api.handle_message.ok_or_else(|| {
            PluginLoaderError::Runtime("plugin has no message handler".to_owned())
        })?;
        let free_buffer = api.free_buffer.ok_or_else(|| {
            PluginLoaderError::Runtime("plugin has no buffer destructor".to_owned())
        })?;
        let mut response = tiktools_plugin_api::PluginBuffer::empty();
        let status =
            unsafe { handle_message(self.context, request.as_ptr(), request.len(), &mut response) };
        let bytes = if status == PluginStatus::Ok {
            match copy_plugin_response(&response) {
                Ok(bytes) => bytes,
                Err(error) => {
                    unsafe { free_buffer(&mut response) };
                    return Err(error);
                }
            }
        } else {
            Vec::new()
        };
        unsafe { free_buffer(&mut response) };
        if status != PluginStatus::Ok {
            return Err(PluginLoaderError::Runtime(format!(
                "plugin call failed: {status:?}"
            )));
        }
        Ok(bytes)
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        let api = unsafe { &*self.api };
        if let Some(destroy) = api.destroy {
            unsafe { destroy(self.context) };
        }
        self.context = std::ptr::null_mut();
        Ok(())
    }
}

#[cfg(all(feature = "native-plugins", not(target_os = "android")))]
fn copy_plugin_response(
    response: &tiktools_plugin_api::PluginBuffer,
) -> Result<Vec<u8>, PluginLoaderError> {
    if response.len > tiktools_plugin_api::MAX_FRAME_BYTES
        || (response.len > 0 && response.ptr.is_null())
    {
        return Err(PluginLoaderError::Runtime(
            "plugin returned an invalid buffer".to_owned(),
        ));
    }
    if response.len == 0 {
        return Ok(Vec::new());
    }

    // SAFETY: the validation above guarantees a non-null pointer and bounds
    // the number of bytes copied. The plugin owns the allocation until the
    // caller invokes its free_buffer function.
    Ok(unsafe { std::slice::from_raw_parts(response.ptr, response.len).to_vec() })
}

#[cfg(all(test, feature = "native-plugins", not(target_os = "android")))]
mod tests {
    use super::*;

    #[test]
    fn copies_empty_response_without_dereferencing_a_null_pointer() {
        let response = tiktools_plugin_api::PluginBuffer::empty();
        assert_eq!(copy_plugin_response(&response).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn rejects_non_empty_response_with_a_null_pointer() {
        let response = tiktools_plugin_api::PluginBuffer {
            ptr: std::ptr::null_mut(),
            len: 1,
            capacity: 0,
        };
        assert!(copy_plugin_response(&response).is_err());
    }

    #[test]
    fn rejects_responses_larger_than_the_frame_limit() {
        let response = tiktools_plugin_api::PluginBuffer {
            ptr: std::ptr::null_mut(),
            len: tiktools_plugin_api::MAX_FRAME_BYTES + 1,
            capacity: 0,
        };
        assert!(copy_plugin_response(&response).is_err());
    }
}
