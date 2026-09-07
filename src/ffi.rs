//! C FFI – exposed as cdylib at /Library/System/coredata.library

use crate::{ManagedObject, PersistentContainer, StoreType};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::Mutex;

static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

fn set_last_error(msg: String) {
    *LAST_ERROR.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg);
}

fn clear_last_error() {
    *LAST_ERROR.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[no_mangle]
pub extern "C" fn coredata_last_error() -> *mut c_char {
    let guard = LAST_ERROR.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = guard.as_ref() {
        if let Ok(cs) = CString::new(s.as_str()) {
            return cs.into_raw();
        }
    }
    ptr::null_mut()
}

/// # Safety
/// `s` must be a pointer returned by `coredata_get`/`coredata_last_error` or null.
#[no_mangle]
pub unsafe extern "C" fn coredata_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub extern "C" fn coredata_version() -> *const c_char {
    c"26.1.0".as_ptr()
}

// Opaque handle to PersistentContainer
pub struct ContainerHandle {
    container: PersistentContainer,
}

/// # Safety
/// Both pointers must be valid nul-terminated C strings or null (null returns error).
#[no_mangle]
pub unsafe extern "C" fn coredata_open(bundle_id: *const c_char, store_type: *const c_char) -> *mut ContainerHandle {
    clear_last_error();
    if bundle_id.is_null() || store_type.is_null() {
        set_last_error("null bundle_id or store_type".into());
        return ptr::null_mut();
    }
    let bundle = CStr::from_ptr(bundle_id).to_string_lossy().to_string();
    let st = CStr::from_ptr(store_type).to_string_lossy().to_string().to_lowercase();
    let store_ty = match st.as_str() {
        "fico" | "fishfile" => StoreType::Fico,
        "sqlite" | "sqlite3" => StoreType::SQLite,
        _ => {
            set_last_error(format!("unknown store type: {}", st));
            return ptr::null_mut();
        }
    };
    match PersistentContainer::new_with_bundle(bundle, store_ty) {
        Ok(c) => Box::into_raw(Box::new(ContainerHandle { container: c })),
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// # Safety
/// `handle` must be a valid pointer from `coredata_open` or null.
#[no_mangle]
pub unsafe extern "C" fn coredata_close(handle: *mut ContainerHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn coredata_save(handle: *mut ContainerHandle) -> c_int {
    if handle.is_null() {
        set_last_error("null handle".into());
        return -1;
    }
    let h = &mut *handle;
    match h.container.save() {
        Ok(()) => { clear_last_error(); 0 }
        Err(e) => { set_last_error(e.to_string()); -1 }
    }
}

/// Set value (stringified via JSON) for entity/id/key
/// # Safety
/// All pointers must be valid C strings or null.
#[no_mangle]
pub unsafe extern "C" fn coredata_set(handle: *mut ContainerHandle, entity: *const c_char, object_id: *const c_char, key: *const c_char, json_value: *const c_char) -> c_int {
    if handle.is_null() || entity.is_null() || object_id.is_null() || key.is_null() || json_value.is_null() {
        set_last_error("null arg".into()); return -1;
    }
    let h = &mut *handle;
    let entity_s = CStr::from_ptr(entity).to_string_lossy().to_string();
    let id_s = CStr::from_ptr(object_id).to_string_lossy().to_string();
    let key_s = CStr::from_ptr(key).to_string_lossy().to_string();
    let val_s = CStr::from_ptr(json_value).to_string_lossy().to_string();

    let json: serde_json::Value = match serde_json::from_str(&val_s) {
        Ok(v) => v,
        Err(_) => serde_json::Value::String(val_s.clone()),
    };
    let fish_val = fishfile::FishValue::from_json(&json);

    let mut ctx = h.container.view_context();
    let mut obj = match ctx.object(&entity_s, &id_s) {
        Ok(o) => o,
        Err(_) => ManagedObject::with_id(entity_s.clone(), id_s.clone()),
    };
    obj.set(key_s, fish_val);
    match ctx.save_object(obj) {
        Ok(()) => { let _ = ctx.save(); clear_last_error(); 0 }
        Err(e) => { set_last_error(e.to_string()); -1 }
    }
}

/// # Safety
/// All pointers must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn coredata_get(handle: *mut ContainerHandle, entity: *const c_char, object_id: *const c_char, key: *const c_char) -> *mut c_char {
    if handle.is_null() || entity.is_null() || object_id.is_null() || key.is_null() {
        set_last_error("null arg".into()); return ptr::null_mut();
    }
    let entity_s = CStr::from_ptr(entity).to_string_lossy().to_string();
    let id_s = CStr::from_ptr(object_id).to_string_lossy().to_string();
    let key_s = CStr::from_ptr(key).to_string_lossy().to_string();
    let h_mut = &mut *handle;
    let ctx2 = h_mut.container.view_context();
    match ctx2.object(&entity_s, &id_s) {
        Ok(obj) => {
            if let Some(v) = obj.get(&key_s) {
                let json = v.to_json();
                if let Ok(s) = serde_json::to_string(&json) {
                    if let Ok(cs) = CString::new(s) { return cs.into_raw(); }
                }
            }
            set_last_error(format!("key not found: {}", key_s));
            ptr::null_mut()
        }
        Err(e) => { set_last_error(e.to_string()); ptr::null_mut() }
    }
}

/// # Safety
/// `handle` and `object_id` must be valid.
#[no_mangle]
pub unsafe extern "C" fn coredata_delete(handle: *mut ContainerHandle, _entity: *const c_char, object_id: *const c_char) -> c_int {
    if handle.is_null() || object_id.is_null() { set_last_error("null arg".into()); return -1; }
    let h = &mut *handle;
    let id_s = CStr::from_ptr(object_id).to_string_lossy().to_string();
    let mut ctx = h.container.view_context();
    match ctx.delete(&id_s) {
        Ok(()) => { let _ = ctx.save(); clear_last_error(); 0 }
        Err(e) => { set_last_error(e.to_string()); -1 }
    }
}

/// # Safety
/// `handle` and `entity` must be valid.
#[no_mangle]
pub unsafe extern "C" fn coredata_fetch_count(handle: *mut ContainerHandle, entity: *const c_char) -> c_int {
    if handle.is_null() || entity.is_null() { return -1; }
    let h = &mut *handle;
    let entity_s = CStr::from_ptr(entity).to_string_lossy().to_string();
    let ctx = h.container.view_context();
    match ctx.fetch_all(&entity_s) {
        Ok(v) => v.len() as c_int,
        Err(e) => { set_last_error(e.to_string()); -1 }
    }
}
