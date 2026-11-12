//! Object-related Node-API entry points, grouped by resource responsibility.
//!
//! Keep this facade stable for the generated dispatch table. Child modules
//! share the existing Env/RAII helpers and retain each entry point's C ABI.
use super::{
    super::{env::Env, types::*},
    is_object,
};
use rquickjs::qjs::{self, JSValue};

mod arrays;
pub(in super::super) use arrays::{
    api_create_array, api_create_array_with_length, api_delete_element, api_get_array_length,
    api_get_element, api_has_element, api_is_array, api_set_element,
};

mod properties;
pub(in super::super) use properties::{
    api_create_object, api_delete_property, api_get_named_property, api_get_property,
    api_get_prototype, api_has_named_property, api_has_own_property, api_has_property,
    api_instanceof, api_object_freeze, api_object_seal, api_set_named_property, api_set_property,
    api_set_prototype,
};

mod keys;
pub(in super::super) use keys::{api_get_all_property_names, api_get_property_names};

mod classes;
pub(in super::super) use classes::{
    api_create_object_with_properties, api_define_class, api_define_properties,
};

mod wrapping;
pub(in super::super) use wrapping::{
    api_add_finalizer, api_check_object_type_tag, api_post_finalizer, api_remove_wrap,
    api_type_tag_object, api_unwrap, api_wrap,
};

mod references;
pub(in super::super) use references::{
    api_create_reference, api_delete_reference, api_get_reference_value, api_reference_ref,
    api_reference_unref,
};

mod scopes;
pub(in super::super) use scopes::{
    api_async_destroy, api_async_init, api_close_callback_scope, api_close_escapable_handle_scope,
    api_close_handle_scope, api_escape_handle, api_open_callback_scope,
    api_open_escapable_handle_scope, api_open_handle_scope,
};

mod lifecycle;
pub(in super::super) use lifecycle::{
    api_add_async_cleanup_hook, api_add_env_cleanup_hook, api_get_instance_data,
    api_remove_async_cleanup_hook, api_remove_env_cleanup_hook, api_set_instance_data,
};

mod environment;
pub(in super::super) use environment::{
    api_adjust_external_memory, api_fatal_error, api_fatal_exception, api_get_last_error_info,
    api_get_module_file_name, api_get_node_version, api_get_uv_event_loop, api_get_version,
    api_module_register, api_run_script,
};

fn object_arg(env: &Env, handle: Handle) -> Result<JSValue, i32> {
    let value = env.get(handle)?;
    if is_object(value) {
        Ok(value)
    } else if unsafe { qjs::JS_IsUndefined(value) || qjs::JS_IsNull(value) } {
        Err(OBJECT_EXPECTED)
    } else {
        Ok(value)
    }
}
