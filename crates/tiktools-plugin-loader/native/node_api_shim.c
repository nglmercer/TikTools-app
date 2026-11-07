#if defined(_MSC_VER)
#include <intrin.h>
#else
#include <stdatomic.h>
#endif
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef void* napi_env;
typedef void* napi_value;
typedef void* napi_callback_info;
typedef void* napi_handle_scope;
typedef void* napi_escapable_handle_scope;
typedef void* napi_deferred;
typedef void* napi_async_work;
typedef void* napi_async_context;
typedef void* napi_callback_scope;
typedef void* napi_threadsafe_function;
typedef void* napi_async_cleanup_hook_handle;
typedef int32_t napi_status;
typedef int32_t napi_typedarray_type;
typedef napi_value (*napi_callback)(napi_env env, napi_callback_info info);
typedef void (*napi_finalize)(napi_env env, void* finalize_data,
                              void* finalize_hint);
typedef void (*node_api_noenv_finalize)(void* finalize_data,
                                        void* finalize_hint);
typedef void (*napi_cleanup_hook)(void* arg);
typedef void (*napi_async_execute_callback)(napi_env env, void* data);
typedef void (*napi_async_complete_callback)(napi_env env, napi_status status,
                                             void* data);
typedef void (*napi_threadsafe_function_call_js)(napi_env env,
                                                  napi_value js_callback,
                                                  void* context, void* data);
typedef void (*napi_async_cleanup_hook)(napi_async_cleanup_hook_handle handle,
                                        void* data);
typedef struct napi_type_tag {
  uint64_t lower;
  uint64_t upper;
} napi_type_tag;
typedef struct napi_extended_error_info {
  const char* error_message;
  void* engine_reserved;
  uint32_t engine_error_code;
  napi_status error_code;
} napi_extended_error_info;

typedef struct tiktools_node_version {
  uint32_t major;
  uint32_t minor;
  uint32_t patch;
  const char* release;
} tiktools_node_version;

typedef struct tiktools_property_descriptor {
  const char* utf8name;
  napi_value name;
  napi_callback method;
  napi_callback getter;
  napi_callback setter;
  napi_value value;
  int32_t attributes;
  void* data;
} tiktools_property_descriptor;

typedef struct tiktools_node_api_table {
  napi_status (*get_undefined)(napi_env, napi_value*);
  napi_status (*get_global)(napi_env, napi_value*);
  napi_status (*get_null)(napi_env, napi_value*);
  napi_status (*get_boolean)(napi_env, bool, napi_value*);
  napi_status (*coerce_to_bool)(napi_env, napi_value, napi_value*);
  napi_status (*coerce_to_number)(napi_env, napi_value, napi_value*);
  napi_status (*coerce_to_string)(napi_env, napi_value, napi_value*);
  napi_status (*create_double)(napi_env, double, napi_value*);
  napi_status (*create_int32)(napi_env, int32_t, napi_value*);
  napi_status (*create_uint32)(napi_env, uint32_t, napi_value*);
  napi_status (*create_int64)(napi_env, int64_t, napi_value*);
  napi_status (*create_string_latin1)(napi_env, const char*, size_t,
                                      napi_value*);
  napi_status (*create_string_utf8)(napi_env, const char*, size_t, napi_value*);
  napi_status (*create_string_utf16)(napi_env, const uint16_t*, size_t,
                                     napi_value*);
  napi_status (*create_symbol)(napi_env, napi_value, napi_value*);
  napi_status (*create_external)(napi_env, void*, napi_finalize, void*,
                                 napi_value*);
  napi_status (*typeof_value)(napi_env, napi_value, int32_t*);
  napi_status (*get_value_external)(napi_env, napi_value, void**);
  napi_status (*get_value_double)(napi_env, napi_value, double*);
  napi_status (*get_value_int32)(napi_env, napi_value, int32_t*);
  napi_status (*get_value_uint32)(napi_env, napi_value, uint32_t*);
  napi_status (*get_value_int64)(napi_env, napi_value, int64_t*);
  napi_status (*get_value_bool)(napi_env, napi_value, bool*);
  napi_status (*get_value_string_latin1)(napi_env, napi_value, char*, size_t,
                                         size_t*);
  napi_status (*get_value_string_utf8)(napi_env, napi_value, char*, size_t,
                                       size_t*);
  napi_status (*get_value_string_utf16)(napi_env, napi_value, uint16_t*,
                                        size_t, size_t*);
  napi_status (*create_array)(napi_env, napi_value*);
  napi_status (*create_array_with_length)(napi_env, size_t, napi_value*);
  napi_status (*is_array)(napi_env, napi_value, bool*);
  napi_status (*get_array_length)(napi_env, napi_value, uint32_t*);
  napi_status (*get_prototype)(napi_env, napi_value, napi_value*);
  napi_status (*get_element)(napi_env, napi_value, uint32_t, napi_value*);
  napi_status (*set_element)(napi_env, napi_value, uint32_t, napi_value);
  napi_status (*has_element)(napi_env, napi_value, uint32_t, bool*);
  napi_status (*create_buffer)(napi_env, size_t, void**, napi_value*);
  napi_status (*create_buffer_copy)(napi_env, size_t, const void*, void**,
                                    napi_value*);
  napi_status (*get_buffer_info)(napi_env, napi_value, void**, size_t*);
  napi_status (*is_buffer)(napi_env, napi_value, bool*);
  napi_status (*is_arraybuffer)(napi_env, napi_value, bool*);
  napi_status (*create_arraybuffer)(napi_env, size_t, void**, napi_value*);
  napi_status (*get_arraybuffer_info)(napi_env, napi_value, void**, size_t*);
  napi_status (*is_typedarray)(napi_env, napi_value, bool*);
  napi_status (*create_typedarray)(napi_env, napi_typedarray_type, size_t,
                                   napi_value, size_t, napi_value*);
  napi_status (*get_typedarray_info)(napi_env, napi_value,
                                     napi_typedarray_type*, size_t*, void**,
                                     napi_value*, size_t*);
  napi_status (*create_dataview)(napi_env, size_t, napi_value, size_t,
                                 napi_value*);
  napi_status (*is_dataview)(napi_env, napi_value, bool*);
  napi_status (*get_dataview_info)(napi_env, napi_value, size_t*, void**,
                                   napi_value*, size_t*);
  napi_status (*create_error)(napi_env, napi_value, napi_value, napi_value*);
  napi_status (*create_type_error)(napi_env, napi_value, napi_value,
                                   napi_value*);
  napi_status (*create_range_error)(napi_env, napi_value, napi_value,
                                    napi_value*);
  napi_status (*throw_value)(napi_env, napi_value);
  napi_status (*throw_error)(napi_env, const char*, const char*);
  napi_status (*throw_type_error)(napi_env, const char*, const char*);
  napi_status (*throw_range_error)(napi_env, const char*, const char*);
  napi_status (*is_exception_pending)(napi_env, bool*);
  napi_status (*get_and_clear_last_exception)(napi_env, napi_value*);
  napi_status (*is_error)(napi_env, napi_value, bool*);
  napi_status (*create_reference)(napi_env, napi_value, uint32_t, void**);
  napi_status (*delete_reference)(napi_env, void*);
  napi_status (*reference_ref)(napi_env, void*, uint32_t*);
  napi_status (*reference_unref)(napi_env, void*, uint32_t*);
  napi_status (*get_reference_value)(napi_env, void*, napi_value*);
  napi_status (*wrap)(napi_env, napi_value, void*, napi_finalize, void*, void**);
  napi_status (*unwrap)(napi_env, napi_value, void**);
  napi_status (*remove_wrap)(napi_env, napi_value, void**);
  napi_status (*create_object)(napi_env, napi_value*);
  napi_status (*define_properties)(napi_env, napi_value, size_t,
                                   const tiktools_property_descriptor*);
  napi_status (*define_class)(napi_env, const char*, size_t, napi_callback,
                              void*, size_t,
                              const tiktools_property_descriptor*,
                              napi_value*);
  napi_status (*create_function)(napi_env, const char*, size_t, napi_callback,
                                 void*, napi_value*);
  napi_status (*set_named_property)(napi_env, napi_value, const char*, napi_value);
  napi_status (*get_named_property)(napi_env, napi_value, const char*, napi_value*);
  napi_status (*get_property)(napi_env, napi_value, napi_value, napi_value*);
  napi_status (*set_property)(napi_env, napi_value, napi_value, napi_value);
  napi_status (*has_property)(napi_env, napi_value, napi_value, bool*);
  napi_status (*delete_property)(napi_env, napi_value, napi_value, bool*);
  napi_status (*delete_element)(napi_env, napi_value, uint32_t, bool*);
  napi_status (*has_own_property)(napi_env, napi_value, napi_value, bool*);
  napi_status (*has_named_property)(napi_env, napi_value, const char*, bool*);
  napi_status (*get_property_names)(napi_env, napi_value, napi_value*);
  napi_status (*call_function)(napi_env, napi_value, napi_value, size_t,
                               const napi_value*, napi_value*);
  napi_status (*new_instance)(napi_env, napi_value, size_t, const napi_value*,
                              napi_value*);
  napi_status (*instanceof)(napi_env, napi_value, napi_value, bool*);
  napi_status (*get_cb_info)(napi_env, napi_callback_info, size_t*, napi_value*,
                             napi_value*, void**);
  napi_status (*open_handle_scope)(napi_env, napi_handle_scope*);
  napi_status (*close_handle_scope)(napi_env, napi_handle_scope);
  napi_status (*open_escapable_handle_scope)(napi_env,
                                             napi_escapable_handle_scope*);
  napi_status (*close_escapable_handle_scope)(napi_env,
                                              napi_escapable_handle_scope);
  napi_status (*escape_handle)(napi_env, napi_escapable_handle_scope,
                               napi_value, napi_value*);
  napi_status (*create_promise)(napi_env, napi_deferred*, napi_value*);
  napi_status (*resolve_deferred)(napi_env, napi_deferred, napi_value);
  napi_status (*reject_deferred)(napi_env, napi_deferred, napi_value);
  napi_status (*is_promise)(napi_env, napi_value, bool*);
  napi_status (*create_async_work)(napi_env, napi_value, napi_value,
                                   napi_async_execute_callback,
                                   napi_async_complete_callback, void*,
                                   napi_async_work*);
  napi_status (*delete_async_work)(napi_env, napi_async_work);
  napi_status (*queue_async_work)(napi_env, napi_async_work);
  napi_status (*cancel_async_work)(napi_env, napi_async_work);
  napi_status (*create_threadsafe_function)(
      napi_env, napi_value, napi_value, napi_value, size_t, size_t, void*,
      napi_finalize, void*, napi_threadsafe_function_call_js,
      napi_threadsafe_function*);
  napi_status (*get_threadsafe_function_context)(napi_threadsafe_function,
                                                 void**);
  napi_status (*call_threadsafe_function)(napi_threadsafe_function, void*,
                                          int32_t);
  napi_status (*acquire_threadsafe_function)(napi_threadsafe_function);
  napi_status (*release_threadsafe_function)(napi_threadsafe_function,
                                             int32_t);
  napi_status (*ref_threadsafe_function)(napi_env,
                                         napi_threadsafe_function);
  napi_status (*unref_threadsafe_function)(napi_env,
                                           napi_threadsafe_function);
  napi_status (*add_env_cleanup_hook)(napi_env, napi_cleanup_hook, void*);
  napi_status (*remove_env_cleanup_hook)(napi_env, napi_cleanup_hook, void*);
  napi_status (*get_last_error_info)(
      napi_env, const napi_extended_error_info**);
  napi_status (*get_new_target)(napi_env, napi_callback_info, napi_value*);
  napi_status (*get_version)(napi_env, uint32_t*);
  napi_status (*strict_equals)(napi_env, napi_value, napi_value, bool*);
  napi_status (*run_script)(napi_env, napi_value, napi_value*);
  napi_status (*adjust_external_memory)(napi_env, int64_t, int64_t*);
  napi_status (*coerce_to_object)(napi_env, napi_value, napi_value*);
  napi_status (*create_external_arraybuffer)(napi_env, void*, size_t,
                                             napi_finalize, void*, napi_value*);
  napi_status (*create_external_buffer)(napi_env, size_t, void*, napi_finalize,
                                        void*, napi_value*);
  napi_status (*async_init)(napi_env, napi_value, napi_value,
                            napi_async_context*);
  napi_status (*async_destroy)(napi_env, napi_async_context);
  napi_status (*make_callback)(napi_env, napi_async_context, napi_value,
                               napi_value, size_t, const napi_value*,
                               napi_value*);
  napi_status (*open_callback_scope)(napi_env, napi_value, napi_async_context,
                                     napi_callback_scope*);
  napi_status (*close_callback_scope)(napi_env, napi_callback_scope);
  napi_status (*create_date)(napi_env, double, napi_value*);
  napi_status (*is_date)(napi_env, napi_value, bool*);
  napi_status (*get_date_value)(napi_env, napi_value, double*);
  napi_status (*add_finalizer)(napi_env, napi_value, void*, napi_finalize,
                               void*, void**);
  napi_status (*create_bigint_int64)(napi_env, int64_t, napi_value*);
  napi_status (*create_bigint_uint64)(napi_env, uint64_t, napi_value*);
  napi_status (*create_bigint_words)(napi_env, int32_t, size_t,
                                     const uint64_t*, napi_value*);
  napi_status (*get_value_bigint_int64)(napi_env, napi_value, int64_t*, bool*);
  napi_status (*get_value_bigint_uint64)(napi_env, napi_value, uint64_t*, bool*);
  napi_status (*get_value_bigint_words)(napi_env, napi_value, int32_t*, size_t*,
                                        uint64_t*);
  napi_status (*get_all_property_names)(napi_env, napi_value, int32_t, int32_t,
                                        int32_t, napi_value*);
  napi_status (*set_instance_data)(napi_env, void*, napi_finalize, void*);
  napi_status (*get_instance_data)(napi_env, void**);
  napi_status (*detach_arraybuffer)(napi_env, napi_value);
  napi_status (*is_detached_arraybuffer)(napi_env, napi_value, bool*);
  napi_status (*type_tag_object)(napi_env, napi_value, const napi_type_tag*);
  napi_status (*check_object_type_tag)(napi_env, napi_value,
                                       const napi_type_tag*, bool*);
  napi_status (*object_freeze)(napi_env, napi_value);
  napi_status (*object_seal)(napi_env, napi_value);
  napi_status (*add_async_cleanup_hook)(napi_env, napi_async_cleanup_hook,
                                        void*, napi_async_cleanup_hook_handle*);
  void (*remove_async_cleanup_hook)(napi_async_cleanup_hook_handle);
  napi_status (*node_api_symbol_for)(napi_env, const char*, size_t, napi_value*);
  napi_status (*create_syntax_error)(napi_env, napi_value, napi_value,
                                     napi_value*);
  napi_status (*throw_syntax_error)(napi_env, const char*, const char*);
  napi_status (*get_module_file_name)(napi_env, const char**);
  napi_status (*create_external_string_latin1)(
      napi_env, char*, size_t, napi_finalize, void*, napi_value*, bool*);
  napi_status (*create_external_string_utf16)(
      napi_env, uint16_t*, size_t, napi_finalize, void*, napi_value*, bool*);
  napi_status (*create_property_key_latin1)(napi_env, const char*, size_t,
                                             napi_value*);
  napi_status (*create_property_key_utf8)(napi_env, const char*, size_t,
                                           napi_value*);
  napi_status (*create_property_key_utf16)(napi_env, const uint16_t*, size_t,
                                            napi_value*);
  napi_status (*create_buffer_from_arraybuffer)(napi_env, napi_value, size_t,
                                                  size_t, napi_value*);
  napi_status (*get_node_version)(napi_env, const tiktools_node_version**);
  napi_status (*get_uv_event_loop)(napi_env, void**);
  void (*module_register)(void*);
  void (*fatal_error)(const char*, size_t, const char*, size_t);
  napi_status (*fatal_exception)(napi_env, napi_value);
  napi_status (*set_prototype)(napi_env, napi_value, napi_value);
  napi_status (*create_object_with_properties)(napi_env, napi_value,
                                                napi_value*, napi_value*,
                                                size_t, napi_value*);
  napi_status (*post_finalizer)(napi_env, napi_finalize, void*, void*);
  napi_status (*create_sharedarraybuffer)(napi_env, size_t, void**,
                                           napi_value*);
  napi_status (*create_external_sharedarraybuffer)(
      napi_env, void*, size_t, node_api_noenv_finalize, void*, napi_value*);
  napi_status (*is_sharedarraybuffer)(napi_env, napi_value, bool*);
} tiktools_node_api_table;

#if defined(_WIN32)
#define TIKTOOLS_EXPORT __declspec(dllexport)
#define TIKTOOLS_NO_RETURN __declspec(noreturn)
#else
#define TIKTOOLS_EXPORT __attribute__((visibility("default")))
#define TIKTOOLS_NO_RETURN __attribute__((noreturn))
#endif

#if defined(_MSC_VER)
// MSVC does not support C11 atomics in its default C mode. Its interlocked
// pointer operations provide the same publication and read ordering here.
static void* volatile api_table;
#else
static _Atomic(const tiktools_node_api_table*) api_table;
#endif

TIKTOOLS_EXPORT void tiktools_install_node_api_table(
    const tiktools_node_api_table* table) {
#if defined(_MSC_VER)
  _InterlockedExchangePointer(&api_table, (void*)table);
#else
  atomic_store_explicit(&api_table, table, memory_order_release);
#endif
}

static const tiktools_node_api_table* get_api_table(void) {
#if defined(_MSC_VER)
  return (const tiktools_node_api_table*)_InterlockedCompareExchangePointer(
      &api_table, NULL, NULL);
#else
  return atomic_load_explicit(&api_table, memory_order_acquire);
#endif
}

TIKTOOLS_EXPORT napi_status napi_get_undefined(napi_env env, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_undefined(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_global(napi_env env, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_global(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_null(napi_env env, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_null(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_boolean(napi_env env, bool value,
                                            napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_boolean(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_coerce_to_bool(napi_env env, napi_value value,
                                                napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->coerce_to_bool(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_coerce_to_number(napi_env env, napi_value value,
                                                  napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->coerce_to_number(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_coerce_to_string(napi_env env, napi_value value,
                                                  napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->coerce_to_string(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_coerce_to_object(napi_env env, napi_value value,
                                                  napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->coerce_to_object(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_double(napi_env env, double value,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_double(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_int32(napi_env env, int32_t value,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_int32(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_uint32(napi_env env, uint32_t value,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_uint32(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_int64(napi_env env, int64_t value,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_int64(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_string_utf8(napi_env env,
                                                    const char* value,
                                                    size_t length,
                                                    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_string_utf8(env, value, length, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_string_latin1(napi_env env,
                                                      const char* value,
                                                      size_t length,
                                                      napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_string_latin1(env, value, length, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_string_utf16(napi_env env,
                                                     const uint16_t* value,
                                                     size_t length,
                                                     napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_string_utf16(env, value, length, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_symbol(napi_env env,
                                               napi_value description,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_symbol(env, description, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_external(
    napi_env env, void* data, napi_finalize finalize_cb, void* finalize_hint,
    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external(env, data, finalize_cb, finalize_hint,
                                         result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_typeof(napi_env env, napi_value value,
                                        int32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->typeof_value(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_external(napi_env env,
                                                    napi_value value,
                                                    void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_external(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_double(napi_env env,
                                                  napi_value value,
                                                  double* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_double(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_int32(napi_env env, napi_value value,
                                                 int32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_int32(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_uint32(napi_env env,
                                                  napi_value value,
                                                  uint32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_uint32(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_int64(napi_env env,
                                                 napi_value value,
                                                 int64_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_int64(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_bool(napi_env env, napi_value value,
                                                bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_bool(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_string_utf8(napi_env env,
                                                        napi_value value,
                                                        char* buffer,
                                                        size_t buffer_size,
                                                        size_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_string_utf8(env, value, buffer, buffer_size,
                                               result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_string_latin1(napi_env env,
                                                         napi_value value,
                                                         char* buffer,
                                                         size_t buffer_size,
                                                         size_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_string_latin1(env, value, buffer, buffer_size,
                                                 result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_string_utf16(napi_env env,
                                                        napi_value value,
                                                        uint16_t* buffer,
                                                        size_t buffer_size,
                                                        size_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_string_utf16(env, value, buffer, buffer_size,
                                                result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_array(napi_env env,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_array(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_array_with_length(napi_env env,
                                                          size_t length,
                                                          napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_array_with_length(env, length, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_array(napi_env env, napi_value value,
                                          bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_array(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_array_length(napi_env env,
                                                  napi_value value,
                                                  uint32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_array_length(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_prototype(napi_env env, napi_value object,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_prototype(env, object, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_element(napi_env env, napi_value value,
                                             uint32_t index,
                                             napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_element(env, value, index, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_set_element(napi_env env, napi_value value,
                                             uint32_t index,
                                             napi_value element) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->set_element(env, value, index, element) : 9;
}

TIKTOOLS_EXPORT napi_status napi_has_element(napi_env env, napi_value value,
                                             uint32_t index, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->has_element(env, value, index, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_buffer(napi_env env, size_t length,
                                               void** data,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_buffer(env, length, data, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_buffer_copy(napi_env env, size_t length,
                                                    const void* data,
                                                    void** result_data,
                                                    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_buffer_copy(env, length, data, result_data,
                                            result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_buffer_info(napi_env env, napi_value value,
                                                 void** data,
                                                 size_t* length) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_buffer_info(env, value, data, length) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_buffer(napi_env env, napi_value value,
                                           bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_buffer(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_arraybuffer(napi_env env, napi_value value,
                                                bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_arraybuffer(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_arraybuffer(napi_env env,
                                                     size_t byte_length,
                                                     void** data,
                                                     napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_arraybuffer(env, byte_length, data, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_external_arraybuffer(
    napi_env env, void* external_data, size_t byte_length,
    napi_finalize finalize_cb, void* finalize_hint, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external_arraybuffer(
                     env, external_data, byte_length, finalize_cb,
                     finalize_hint, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_external_buffer(
    napi_env env, size_t length, void* data, napi_finalize finalize_cb,
    void* finalize_hint, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external_buffer(env, length, data, finalize_cb,
                                                finalize_hint, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_async_init(napi_env env,
                                            napi_value async_resource,
                                            napi_value async_resource_name,
                                            napi_async_context* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->async_init(env, async_resource, async_resource_name,
                                    result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_async_destroy(napi_env env,
                                              napi_async_context async_context) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->async_destroy(env, async_context) : 9;
}

TIKTOOLS_EXPORT napi_status napi_make_callback(
    napi_env env, napi_async_context async_context, napi_value recv,
    napi_value func, size_t argc, const napi_value* argv, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->make_callback(env, async_context, recv, func, argc,
                                       argv, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_open_callback_scope(
    napi_env env, napi_value resource_object, napi_async_context async_context,
    napi_callback_scope* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->open_callback_scope(env, resource_object, async_context,
                                             result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_close_callback_scope(
    napi_env env, napi_callback_scope scope) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->close_callback_scope(env, scope) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_date(napi_env env, double time,
                                             napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_date(env, time, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_date(napi_env env, napi_value value,
                                         bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_date(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_date_value(napi_env env, napi_value value,
                                                double* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_date_value(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_add_finalizer(
    napi_env env, napi_value object, void* finalize_data,
    napi_finalize finalize_cb, void* finalize_hint, void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->add_finalizer(env, object, finalize_data, finalize_cb,
                                       finalize_hint, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_bigint_int64(napi_env env,
                                                      int64_t value,
                                                      napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_bigint_int64(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_bigint_uint64(napi_env env,
                                                       uint64_t value,
                                                       napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_bigint_uint64(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_bigint_words(napi_env env,
                                                      int32_t sign_bit,
                                                      size_t word_count,
                                                      const uint64_t* words,
                                                      napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_bigint_words(env, sign_bit, word_count, words,
                                             result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_bigint_int64(napi_env env,
                                                         napi_value value,
                                                         int64_t* result,
                                                         bool* lossless) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_bigint_int64(env, value, result, lossless)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_bigint_uint64(napi_env env,
                                                          napi_value value,
                                                          uint64_t* result,
                                                          bool* lossless) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_bigint_uint64(env, value, result, lossless)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_value_bigint_words(napi_env env,
                                                         napi_value value,
                                                         int32_t* sign_bit,
                                                         size_t* word_count,
                                                         uint64_t* words) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_value_bigint_words(env, value, sign_bit, word_count,
                                                words)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_all_property_names(
    napi_env env, napi_value object, int32_t key_mode, int32_t key_filter,
    int32_t key_conversion, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_all_property_names(env, object, key_mode, key_filter,
                                                key_conversion, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_set_instance_data(napi_env env, void* data,
                                                   napi_finalize finalize_cb,
                                                   void* finalize_hint) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->set_instance_data(env, data, finalize_cb, finalize_hint)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_instance_data(napi_env env, void** data) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_instance_data(env, data) : 9;
}

TIKTOOLS_EXPORT napi_status napi_detach_arraybuffer(napi_env env,
                                                     napi_value arraybuffer) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->detach_arraybuffer(env, arraybuffer) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_detached_arraybuffer(napi_env env,
                                                          napi_value arraybuffer,
                                                          bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_detached_arraybuffer(env, arraybuffer, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_arraybuffer_info(napi_env env,
                                                      napi_value arraybuffer,
                                                      void** data,
                                                      size_t* byte_length) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_arraybuffer_info(env, arraybuffer, data,
                                              byte_length)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_typedarray(napi_env env, napi_value value,
                                               bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_typedarray(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_typedarray(
    napi_env env, napi_typedarray_type type, size_t length,
    napi_value arraybuffer, size_t byte_offset, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_typedarray(env, type, length, arraybuffer,
                                           byte_offset, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_typedarray_info(
    napi_env env, napi_value typedarray, napi_typedarray_type* type,
    size_t* length, void** data, napi_value* arraybuffer,
    size_t* byte_offset) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_typedarray_info(env, typedarray, type, length,
                                             data, arraybuffer, byte_offset)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_dataview(
    napi_env env, size_t byte_length, napi_value arraybuffer,
    size_t byte_offset, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_dataview(env, byte_length, arraybuffer,
                                         byte_offset, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_dataview(napi_env env, napi_value value,
                                             bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_dataview(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_dataview_info(
    napi_env env, napi_value dataview, size_t* byte_length, void** data,
    napi_value* arraybuffer, size_t* byte_offset) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_dataview_info(env, dataview, byte_length, data,
                                            arraybuffer, byte_offset)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_error(napi_env env, napi_value code,
                                              napi_value message,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_error(env, code, message, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_type_error(napi_env env,
                                                   napi_value code,
                                                   napi_value message,
                                                   napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_type_error(env, code, message, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_range_error(napi_env env,
                                                    napi_value code,
                                                    napi_value message,
                                                    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_range_error(env, code, message, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_throw(napi_env env, napi_value error) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->throw_value(env, error) : 9;
}

TIKTOOLS_EXPORT napi_status napi_throw_error(napi_env env, const char* code,
                                             const char* message) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->throw_error(env, code, message) : 9;
}

TIKTOOLS_EXPORT napi_status napi_throw_type_error(napi_env env,
                                                  const char* code,
                                                  const char* message) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->throw_type_error(env, code, message) : 9;
}

TIKTOOLS_EXPORT napi_status napi_throw_range_error(napi_env env,
                                                   const char* code,
                                                   const char* message) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->throw_range_error(env, code, message) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_exception_pending(napi_env env,
                                                      bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_exception_pending(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_and_clear_last_exception(
    napi_env env, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_and_clear_last_exception(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_error(napi_env env, napi_value value,
                                          bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_error(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_reference(napi_env env,
                                                  napi_value value,
                                                  uint32_t initial_ref_count,
                                                  void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_reference(env, value, initial_ref_count, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_delete_reference(napi_env env, void* reference) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->delete_reference(env, reference) : 9;
}

TIKTOOLS_EXPORT napi_status napi_reference_ref(napi_env env, void* reference,
                                               uint32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->reference_ref(env, reference, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_reference_unref(napi_env env, void* reference,
                                                 uint32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->reference_unref(env, reference, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_reference_value(napi_env env,
                                                     void* reference,
                                                     napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_reference_value(env, reference, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_wrap(napi_env env, napi_value object,
                                     void* native_object,
                                     napi_finalize finalize_cb,
                                     void* finalize_hint, void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->wrap(env, object, native_object, finalize_cb,
                             finalize_hint, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_unwrap(napi_env env, napi_value object,
                                       void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->unwrap(env, object, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_remove_wrap(napi_env env, napi_value object,
                                            void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->remove_wrap(env, object, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_object(napi_env env, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_object(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_define_properties(
    napi_env env, napi_value object, size_t property_count,
    const tiktools_property_descriptor* properties) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->define_properties(env, object, property_count, properties)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_define_class(
    napi_env env, const char* utf8name, size_t name_length,
    napi_callback constructor, void* data, size_t property_count,
    const tiktools_property_descriptor* properties, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->define_class(env, utf8name, name_length, constructor,
                                     data, property_count, properties, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_function(
    napi_env env, const char* name, size_t length, napi_callback callback,
    void* data, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_function(env, name, length, callback, data, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_set_named_property(napi_env env,
                                                    napi_value object,
                                                    const char* name,
                                                    napi_value value) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->set_named_property(env, object, name, value) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_named_property(napi_env env,
                                                    napi_value object,
                                                    const char* name,
                                                    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_named_property(env, object, name, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_property(napi_env env, napi_value object,
                                              napi_value key,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_property(env, object, key, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_set_property(napi_env env, napi_value object,
                                              napi_value key,
                                              napi_value value) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->set_property(env, object, key, value) : 9;
}

TIKTOOLS_EXPORT napi_status napi_has_property(napi_env env, napi_value object,
                                              napi_value key, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->has_property(env, object, key, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_delete_property(napi_env env,
                                                 napi_value object,
                                                 napi_value key, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->delete_property(env, object, key, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_delete_element(napi_env env,
                                                napi_value object,
                                                uint32_t index, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->delete_element(env, object, index, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_has_own_property(napi_env env,
                                                 napi_value object,
                                                 napi_value key, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->has_own_property(env, object, key, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_has_named_property(napi_env env,
                                                    napi_value object,
                                                    const char* name,
                                                    bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->has_named_property(env, object, name, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_property_names(napi_env env,
                                                    napi_value object,
                                                    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_property_names(env, object, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_call_function(napi_env env, napi_value recv,
                                               napi_value function, size_t argc,
                                               const napi_value* argv,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->call_function(env, recv, function, argc, argv, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_new_instance(napi_env env,
                                              napi_value constructor,
                                              size_t argc,
                                              const napi_value* argv,
                                              napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->new_instance(env, constructor, argc, argv, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_instanceof(napi_env env, napi_value object,
                                            napi_value constructor,
                                            bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->instanceof(env, object, constructor, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_cb_info(napi_env env,
                                             napi_callback_info info,
                                             size_t* argc, napi_value* argv,
                                             napi_value* this_arg, void** data) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_cb_info(env, info, argc, argv, this_arg, data) : 9;
}

TIKTOOLS_EXPORT napi_status napi_open_handle_scope(napi_env env,
                                                   napi_handle_scope* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->open_handle_scope(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_close_handle_scope(napi_env env,
                                                     napi_handle_scope scope) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->close_handle_scope(env, scope) : 9;
}

TIKTOOLS_EXPORT napi_status napi_open_escapable_handle_scope(
    napi_env env, napi_escapable_handle_scope* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->open_escapable_handle_scope(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_close_escapable_handle_scope(
    napi_env env, napi_escapable_handle_scope scope) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->close_escapable_handle_scope(env, scope) : 9;
}

TIKTOOLS_EXPORT napi_status napi_escape_handle(napi_env env,
                                               napi_escapable_handle_scope scope,
                                               napi_value escapee,
                                               napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->escape_handle(env, scope, escapee, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_promise(napi_env env,
                                                napi_deferred* deferred,
                                                napi_value* promise) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_promise(env, deferred, promise) : 9;
}

TIKTOOLS_EXPORT napi_status napi_resolve_deferred(napi_env env,
                                                  napi_deferred deferred,
                                                  napi_value resolution) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->resolve_deferred(env, deferred, resolution) : 9;
}

TIKTOOLS_EXPORT napi_status napi_reject_deferred(napi_env env,
                                                 napi_deferred deferred,
                                                 napi_value rejection) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->reject_deferred(env, deferred, rejection) : 9;
}

TIKTOOLS_EXPORT napi_status napi_is_promise(napi_env env, napi_value value,
                                            bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_promise(env, value, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_async_work(
    napi_env env, napi_value async_resource, napi_value async_resource_name,
    napi_async_execute_callback execute_callback,
    napi_async_complete_callback complete_callback, void* data,
    napi_async_work* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_async_work(
                     env, async_resource, async_resource_name,
                     execute_callback, complete_callback, data, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_delete_async_work(napi_env env,
                                                   napi_async_work work) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->delete_async_work(env, work) : 9;
}

TIKTOOLS_EXPORT napi_status napi_queue_async_work(napi_env env,
                                                  napi_async_work work) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->queue_async_work(env, work) : 9;
}

TIKTOOLS_EXPORT napi_status napi_cancel_async_work(napi_env env,
                                                   napi_async_work work) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->cancel_async_work(env, work) : 9;
}

TIKTOOLS_EXPORT napi_status napi_create_threadsafe_function(
    napi_env env, napi_value func, napi_value async_resource,
    napi_value async_resource_name, size_t max_queue_size,
    size_t initial_thread_count, void* thread_finalize_data,
    napi_finalize thread_finalize_cb, void* context,
    napi_threadsafe_function_call_js call_js_cb,
    napi_threadsafe_function* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_threadsafe_function(
                     env, func, async_resource, async_resource_name,
                     max_queue_size, initial_thread_count,
                     thread_finalize_data, thread_finalize_cb, context,
                     call_js_cb, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_threadsafe_function_context(
    napi_threadsafe_function function, void** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_threadsafe_function_context(function, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_call_threadsafe_function(
    napi_threadsafe_function function, void* data, int32_t call_mode) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->call_threadsafe_function(function, data, call_mode) : 9;
}

TIKTOOLS_EXPORT napi_status napi_acquire_threadsafe_function(
    napi_threadsafe_function function) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->acquire_threadsafe_function(function) : 9;
}

TIKTOOLS_EXPORT napi_status napi_release_threadsafe_function(
    napi_threadsafe_function function, int32_t release_mode) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->release_threadsafe_function(function, release_mode) : 9;
}

TIKTOOLS_EXPORT napi_status napi_ref_threadsafe_function(
    napi_env env, napi_threadsafe_function function) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->ref_threadsafe_function(env, function) : 9;
}

TIKTOOLS_EXPORT napi_status napi_unref_threadsafe_function(
    napi_env env, napi_threadsafe_function function) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->unref_threadsafe_function(env, function) : 9;
}

TIKTOOLS_EXPORT napi_status napi_add_env_cleanup_hook(
    napi_env env, napi_cleanup_hook fun, void* arg) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->add_env_cleanup_hook(env, fun, arg) : 9;
}

TIKTOOLS_EXPORT napi_status napi_remove_env_cleanup_hook(
    napi_env env, napi_cleanup_hook fun, void* arg) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->remove_env_cleanup_hook(env, fun, arg) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_last_error_info(
    napi_env env, const napi_extended_error_info** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_last_error_info(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_new_target(napi_env env,
                                                napi_callback_info info,
                                                napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_new_target(env, info, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_version(napi_env env, uint32_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_version(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_node_version(
    napi_env env, const tiktools_node_version** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_node_version(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_get_uv_event_loop(napi_env env, void** loop) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_uv_event_loop(env, loop) : 9;
}

TIKTOOLS_EXPORT void napi_module_register(void* module) {
  const tiktools_node_api_table* table = get_api_table();
  if (table) table->module_register(module);
}

TIKTOOLS_EXPORT TIKTOOLS_NO_RETURN void napi_fatal_error(
    const char* location, size_t location_length, const char* message,
    size_t message_length) {
  const tiktools_node_api_table* table = get_api_table();
  if (table) {
    table->fatal_error(location, location_length, message, message_length);
  }
  abort();
}

TIKTOOLS_EXPORT napi_status napi_fatal_exception(napi_env env, napi_value error) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->fatal_exception(env, error) : 9;
}

TIKTOOLS_EXPORT napi_status napi_strict_equals(napi_env env, napi_value lhs,
                                               napi_value rhs, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->strict_equals(env, lhs, rhs, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_run_script(napi_env env, napi_value script,
                                             napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->run_script(env, script, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_adjust_external_memory(
    napi_env env, int64_t change_in_bytes, int64_t* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->adjust_external_memory(env, change_in_bytes, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status napi_type_tag_object(
    napi_env env, napi_value object, const napi_type_tag* type_tag) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->type_tag_object(env, object, type_tag) : 9;
}

TIKTOOLS_EXPORT napi_status napi_check_object_type_tag(
    napi_env env, napi_value object, const napi_type_tag* type_tag,
    bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->check_object_type_tag(env, object, type_tag, result) : 9;
}

TIKTOOLS_EXPORT napi_status napi_object_freeze(napi_env env, napi_value object) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->object_freeze(env, object) : 9;
}

TIKTOOLS_EXPORT napi_status napi_object_seal(napi_env env, napi_value object) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->object_seal(env, object) : 9;
}

TIKTOOLS_EXPORT napi_status napi_add_async_cleanup_hook(
    napi_env env, napi_async_cleanup_hook hook, void* arg,
    napi_async_cleanup_hook_handle* remove_handle) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->add_async_cleanup_hook(env, hook, arg, remove_handle) : 9;
}

TIKTOOLS_EXPORT void napi_remove_async_cleanup_hook(
    napi_async_cleanup_hook_handle remove_handle) {
  const tiktools_node_api_table* table = get_api_table();
  if (table) table->remove_async_cleanup_hook(remove_handle);
}

TIKTOOLS_EXPORT napi_status node_api_symbol_for(
    napi_env env, const char* description, size_t length, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->node_api_symbol_for(env, description, length, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_syntax_error(
    napi_env env, napi_value code, napi_value message, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_syntax_error(env, code, message, result) : 9;
}

TIKTOOLS_EXPORT napi_status node_api_throw_syntax_error(
    napi_env env, const char* code, const char* message) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->throw_syntax_error(env, code, message) : 9;
}

TIKTOOLS_EXPORT napi_status node_api_get_module_file_name(
    napi_env env, const char** result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->get_module_file_name(env, result) : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_external_string_latin1(
    napi_env env, char* str, size_t length, napi_finalize finalize_callback,
    void* finalize_hint, napi_value* result, bool* copied) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external_string_latin1(
                     env, str, length, finalize_callback, finalize_hint, result,
                     copied)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_external_string_utf16(
    napi_env env, uint16_t* str, size_t length,
    napi_finalize finalize_callback, void* finalize_hint, napi_value* result,
    bool* copied) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external_string_utf16(
                     env, str, length, finalize_callback, finalize_hint, result,
                     copied)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_property_key_latin1(
    napi_env env, const char* str, size_t length, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_property_key_latin1(env, str, length, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_property_key_utf8(
    napi_env env, const char* str, size_t length, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_property_key_utf8(env, str, length, result) : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_property_key_utf16(
    napi_env env, const uint16_t* str, size_t length, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_property_key_utf16(env, str, length, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_buffer_from_arraybuffer(
    napi_env env, napi_value arraybuffer, size_t byte_offset,
    size_t byte_length, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_buffer_from_arraybuffer(
                     env, arraybuffer, byte_offset, byte_length, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_set_prototype(napi_env env,
                                                   napi_value object,
                                                   napi_value value) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->set_prototype(env, object, value) : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_object_with_properties(
    napi_env env, napi_value prototype_or_null, napi_value* property_names,
    napi_value* property_values, size_t property_count, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_object_with_properties(
                     env, prototype_or_null, property_names, property_values,
                     property_count, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_post_finalizer(napi_env env,
                                                    napi_finalize finalize_cb,
                                                    void* finalize_data,
                                                    void* finalize_hint) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->post_finalizer(env, finalize_cb, finalize_data,
                                       finalize_hint)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_sharedarraybuffer(
    napi_env env, size_t byte_length, void** data, napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_sharedarraybuffer(env, byte_length, data, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_create_external_sharedarraybuffer(
    napi_env env, void* external_data, size_t byte_length,
    node_api_noenv_finalize finalize_callback, void* finalize_hint,
    napi_value* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->create_external_sharedarraybuffer(
                     env, external_data, byte_length, finalize_callback,
                     finalize_hint, result)
               : 9;
}

TIKTOOLS_EXPORT napi_status node_api_is_sharedarraybuffer(
    napi_env env, napi_value value, bool* result) {
  const tiktools_node_api_table* table = get_api_table();
  return table ? table->is_sharedarraybuffer(env, value, result) : 9;
}
