use std::{
    ffi::{c_char, CStr, CString},
    panic::catch_unwind,
    ptr::{self, null},
};

#[no_mangle]
pub extern "C" fn hell_bad(name: *const c_char) -> *const c_char {
    if name.is_null() {
        return ptr::null();
    }

    let result = catch_unwind(|| {
        if let Ok(s) = unsafe { CStr::from_ptr(name).to_str() } {
            let result = format!("hello {s}!");
            let s = CString::new(result).unwrap();
            s.into_raw()
        } else {
            ptr::null()
        }
    });

    match result {
        Ok(s) => s,
        _ => ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn free_str(s: *mut c_char) {
    if !s.is_null() {
        unsafe { CString::from_raw(s) };
    }
}
