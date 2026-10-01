use anu::nam_args_api::NamSignatureContract;
use anyhow::{Result, anyhow};

#[inline]
pub fn resolve_nam_name<F: 'static>() -> Result<String> {
    let full_name = std::any::type_name::<F>();
    extract_nam_name(full_name)
}

pub fn extract_nam_name(full_name: &str) -> Result<String> {
    let base = match full_name.find('<') {
        Some(idx) => &full_name[..idx],
        None => full_name,
    };

    let last_segment = match base.rfind("::") {
        Some(idx) => &base[idx + 2..],
        None => base,
    };

    if last_segment.is_empty() {
        return Err(anyhow!(
            "[Enki Contract] Failed to extract nam name from identifier: '{full_name}'"
        ));
    }

    Ok(last_segment.to_string())
}

pub fn lookup_signature_contract(nam_name: &str) -> Option<&'static NamSignatureContract> {
    let symbol_name = format!("__ENKI_CONTRACT_{nam_name}\0");

    #[cfg(unix)]
    unsafe {
        unsafe extern "C" {
            fn dlsym(
                handle: *mut std::ffi::c_void,
                symbol: *const std::os::raw::c_char,
            ) -> *mut std::ffi::c_void;
        }
        let ptr = dlsym(
            std::ptr::null_mut(),
            symbol_name.as_ptr() as *const std::os::raw::c_char,
        );
        if !ptr.is_null() {
            return Some(&*(ptr as *const NamSignatureContract));
        }
    }

    #[cfg(windows)]
    unsafe {
        unsafe extern "system" {
            fn GetModuleHandleA(lpModuleName: *const std::os::raw::c_char)
            -> *mut std::ffi::c_void;
            fn GetProcAddress(
                hModule: *mut std::ffi::c_void,
                lpProcName: *const std::os::raw::c_char,
            ) -> *mut std::ffi::c_void;
        }
        let module = GetModuleHandleA(std::ptr::null());
        if !module.is_null() {
            let ptr = GetProcAddress(module, symbol_name.as_ptr() as *const std::os::raw::c_char);
            if !ptr.is_null() {
                return Some(&*(ptr as *const NamSignatureContract));
            }
        }
    }

    None
}
