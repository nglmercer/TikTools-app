//! C ABI types shared with the Node-API shim.

use std::ffi::{c_char, c_void};

use super::env::Env;

pub(super) type Environment = *mut Env;
pub(super) type Handle = *mut c_void;
pub(super) type CbInfo = *mut c_void;
pub(super) type Opaque = *mut c_void;

pub(super) type Callback = Option<unsafe extern "C" fn(Environment, CbInfo) -> Handle>;
pub(super) type Finalize = Option<unsafe extern "C" fn(Environment, *mut c_void, *mut c_void)>;
pub(super) type NoEnvFinalize = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;
pub(super) type CleanupHook = Option<unsafe extern "C" fn(*mut c_void)>;
pub(super) type AsyncExecute = Option<unsafe extern "C" fn(Environment, *mut c_void)>;
pub(super) type AsyncComplete = Option<unsafe extern "C" fn(Environment, i32, *mut c_void)>;
pub(super) type TsfnCallJs =
    Option<unsafe extern "C" fn(Environment, Handle, *mut c_void, *mut c_void)>;
pub(super) type AsyncCleanupHook = Option<unsafe extern "C" fn(Opaque, *mut c_void)>;

#[repr(C)]
pub(super) struct ExtendedErrorInfo {
    pub error_message: *const c_char,
    pub engine_reserved: *mut c_void,
    pub engine_error_code: u32,
    pub error_code: i32,
}

#[repr(C)]
pub(super) struct NodeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub release: *const c_char,
}

#[repr(C)]
pub(super) struct PropertyDescriptor {
    pub utf8name: *const c_char,
    pub name: Handle,
    pub method: Callback,
    pub getter: Callback,
    pub setter: Callback,
    pub value: Handle,
    pub attributes: i32,
    pub data: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct TypeTag {
    pub lower: u64,
    pub upper: u64,
}

// Status codes (napi_status).
pub(super) const OK: i32 = 0;
pub(super) const INVALID_ARG: i32 = 1;
pub(super) const OBJECT_EXPECTED: i32 = 2;
pub(super) const STRING_EXPECTED: i32 = 3;
pub(super) const NAME_EXPECTED: i32 = 4;
pub(super) const FUNCTION_EXPECTED: i32 = 5;
pub(super) const NUMBER_EXPECTED: i32 = 6;
pub(super) const BOOLEAN_EXPECTED: i32 = 7;
pub(super) const ARRAY_EXPECTED: i32 = 8;
pub(super) const GENERIC_FAILURE: i32 = 9;
pub(super) const PENDING_EXCEPTION: i32 = 10;
pub(super) const CANCELLED: i32 = 11;
pub(super) const ESCAPE_CALLED_TWICE: i32 = 12;
pub(super) const HANDLE_SCOPE_MISMATCH: i32 = 13;
#[allow(dead_code)] // part of the complete napi_status table
pub(super) const CALLBACK_SCOPE_MISMATCH: i32 = 14;
pub(super) const QUEUE_FULL: i32 = 15;
pub(super) const CLOSING: i32 = 16;
pub(super) const BIGINT_EXPECTED: i32 = 17;
pub(super) const DATE_EXPECTED: i32 = 18;
pub(super) const ARRAYBUFFER_EXPECTED: i32 = 19;
pub(super) const DETACHABLE_ARRAYBUFFER_EXPECTED: i32 = 20;
#[allow(dead_code)] // part of the complete napi_status table
pub(super) const WOULD_DEADLOCK: i32 = 21;
#[allow(dead_code)] // part of the complete napi_status table
pub(super) const NO_EXTERNAL_BUFFERS_ALLOWED: i32 = 22;
pub(super) const CANNOT_RUN_JS: i32 = 23;

/// `NAPI_AUTO_LENGTH`.
pub(super) const AUTO_LENGTH: usize = usize::MAX;

// napi_valuetype
pub(super) const TYPE_UNDEFINED: i32 = 0;
pub(super) const TYPE_NULL: i32 = 1;
pub(super) const TYPE_BOOLEAN: i32 = 2;
pub(super) const TYPE_NUMBER: i32 = 3;
pub(super) const TYPE_STRING: i32 = 4;
pub(super) const TYPE_SYMBOL: i32 = 5;
pub(super) const TYPE_OBJECT: i32 = 6;
pub(super) const TYPE_FUNCTION: i32 = 7;
pub(super) const TYPE_EXTERNAL: i32 = 8;
pub(super) const TYPE_BIGINT: i32 = 9;

// napi_property_attributes
pub(super) const PROP_WRITABLE: i32 = 1;
pub(super) const PROP_ENUMERABLE: i32 = 2;
pub(super) const PROP_CONFIGURABLE: i32 = 4;
pub(super) const PROP_STATIC: i32 = 1 << 10;
