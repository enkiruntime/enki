use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ParsuArgDescriptorC {
    pub mode: u32,
    pub domain: u32,
    pub intent: u32,
    pub is_optional: u8,
    pub element_size: usize,
    pub stride: usize,
    pub alignment: usize,
    pub arena_size_bytes: usize,
}

#[repr(C)]
struct ParsuSpanC {
    file_path: *mut c_char,
    line: u32,
    column: u32,
    length: u32,
    role: u32,
}

#[repr(C)]
struct ParsuDiagnosticC {
    code: u32,
    spans: *mut ParsuSpanC,
    span_count: usize,
}

#[repr(C)]
struct ParsuCompiledNamC {
    nam_name: *mut c_char,
    spirv_bytecode: *mut u32,
    spirv_word_count: usize,
    stack_size_per_thread: u32,
}

#[repr(C)]
struct ParsuResultC {
    success: bool,
    error_message: *mut c_char,
    nams: *mut ParsuCompiledNamC,
    nam_count: usize,
    diagnostics: *mut ParsuDiagnosticC,
    diagnostic_count: usize,
}

type CompileFn = unsafe extern "C" fn(
    bitcode_path: *const c_char,
    target_nam_name: *const c_char,
    project_root: *const c_char,
    local_x: i32,
    local_y: i32,
    local_z: i32,
    args: *const ParsuArgDescriptorC,
    arg_count: usize,
) -> *mut ParsuResultC;

type FreeFn = unsafe extern "C" fn(result: *mut ParsuResultC);

struct ParsuBackend {
    _lib: Library,
    compile_bitcode: CompileFn,
    free_result: FreeFn,
}

static BACKEND: OnceLock<Result<ParsuBackend, ParsuError>> = OnceLock::new();

fn resolve_lib_path() -> Result<PathBuf, String> {
    crate::bootstrapper::ensure_parsu_ready()
}

fn get_backend() -> Result<&'static ParsuBackend, ParsuError> {
    let backend_res = BACKEND.get_or_init(|| {
        let lib_path = resolve_lib_path().map_err(|e| ParsuError::Generic(e))?;

        #[cfg(target_os = "windows")]
        unsafe {
            if let Some(parent) = lib_path.parent() {
                use std::os::windows::ffi::OsStrExt;
                let wide: Vec<u16> = parent.as_os_str().encode_wide().chain(Some(0)).collect();
                unsafe extern "system" {
                    fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
                }
                SetDllDirectoryW(wide.as_ptr());
            }
        }

        unsafe {
            let lib = Library::new(&lib_path).map_err(|e| {
                ParsuError::Generic(format!(
                    "Failed to load toolchain library at {:?}: {}\n",
                    lib_path, e
                ))
            })?;
            let compile_sym: Symbol<CompileFn> =
                lib.get(b"parsu_compile_bitcode\0").map_err(|e| {
                    ParsuError::Generic(format!("Missing symbol 'parsu_compile_bitcode': {}", e))
                })?;

            let free_sym: Symbol<FreeFn> = lib.get(b"parsu_free_result\0").map_err(|e| {
                ParsuError::Generic(format!("Missing symbol 'parsu_free_result': {}", e))
            })?;

            Ok(ParsuBackend {
                compile_bitcode: *compile_sym,
                free_result: *free_sym,
                _lib: lib,
            })
        }
    });

    match backend_res {
        Ok(backend) => Ok(backend),
        Err(err) => Err(err.clone()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSpan {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub length: u32,
    pub role: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDiagnostic {
    pub code: u32,
    pub spans: Vec<DiagnosticSpan>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ParsuError {
    Diagnostics(Vec<GpuDiagnostic>),
    ToolchainMissing,
    Generic(String),
}

impl fmt::Display for ParsuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Diagnostics(diags) => {
                write!(
                    f,
                    "GPU validation failed with {} diagnostic(s)",
                    diags.len()
                )
            }
            Self::ToolchainMissing => write!(f, "Enki GPU JIT compiler toolchain is missing"),
            Self::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ParsuError {}

#[derive(Debug, Clone)]
pub struct CompiledNam {
    pub nam_name: String,
    pub spirv_bytecode: Vec<u32>,
    pub stack_size_per_thread: u32,
}

pub struct ParsuCompiler;

impl ParsuCompiler {
    pub fn compile_bitcode(
        bitcode_path: &str,
        target_nam_name: &str,
        project_root: &str,
        local_size: (i32, i32, i32),
        descriptors: &[ParsuArgDescriptorC],
    ) -> Result<Vec<CompiledNam>, ParsuError> {
        let backend = get_backend()?;

        let c_bitcode_path =
            CString::new(bitcode_path).map_err(|e| ParsuError::Generic(e.to_string()))?;
        let c_target_nam =
            CString::new(target_nam_name).map_err(|e| ParsuError::Generic(e.to_string()))?;
        let c_project_root =
            CString::new(project_root).map_err(|e| ParsuError::Generic(e.to_string()))?;

        unsafe {
            let raw_res = (backend.compile_bitcode)(
                c_bitcode_path.as_ptr(),
                c_target_nam.as_ptr(),
                c_project_root.as_ptr(),
                local_size.0,
                local_size.1,
                local_size.2,
                descriptors.as_ptr(),
                descriptors.len(),
            );

            if raw_res.is_null() {
                return Err(ParsuError::Generic(
                    "Failed to get response from C++ Compiler Pipeline".to_string(),
                ));
            }

            let res_ref = &*raw_res;

            if !res_ref.success {
                let err_msg = if !res_ref.error_message.is_null() {
                    Some(
                        CStr::from_ptr(res_ref.error_message)
                            .to_string_lossy()
                            .into_owned(),
                    )
                } else {
                    None
                };

                if !res_ref.diagnostics.is_null() && res_ref.diagnostic_count > 0 {
                    let diags_slice =
                        std::slice::from_raw_parts(res_ref.diagnostics, res_ref.diagnostic_count);
                    let mut diagnostics = Vec::with_capacity(diags_slice.len());

                    for d in diags_slice {
                        let mut spans = Vec::new();
                        if !d.spans.is_null() && d.span_count > 0 {
                            let spans_slice = std::slice::from_raw_parts(d.spans, d.span_count);
                            for s in spans_slice {
                                spans.push(DiagnosticSpan {
                                    file_path: c_ptr_to_string(s.file_path, ""),
                                    line: s.line,
                                    column: s.column,
                                    length: s.length,
                                    role: s.role,
                                });
                            }
                        }

                        let msg = if d.code == 9999 {
                            err_msg.clone()
                        } else {
                            None
                        };

                        diagnostics.push(GpuDiagnostic {
                            code: d.code,
                            spans,
                            message: msg,
                        });
                    }

                    (backend.free_result)(raw_res);
                    return Err(ParsuError::Diagnostics(diagnostics));
                }

                if let Some(ref msg) = err_msg {
                    if msg.contains("BEGIN PARSU CRASH TOKEN") {
                        (backend.free_result)(raw_res);
                        return Err(ParsuError::Diagnostics(vec![GpuDiagnostic {
                            code: 9999,
                            spans: Vec::new(),
                            message: err_msg,
                        }]));
                    }
                }

                let final_err = err_msg.unwrap_or_else(|| "Unknown compilation error".to_string());
                (backend.free_result)(raw_res);
                return Err(ParsuError::Generic(final_err));
            }

            let mut compiled_nams = Vec::with_capacity(res_ref.nam_count);

            if !res_ref.nams.is_null() && res_ref.nam_count > 0 {
                let nams_slice = std::slice::from_raw_parts(res_ref.nams, res_ref.nam_count);

                for k in nams_slice {
                    let name = if !k.nam_name.is_null() {
                        CStr::from_ptr(k.nam_name).to_string_lossy().into_owned()
                    } else {
                        "unnamed_nam".to_string()
                    };

                    let bytecode = if !k.spirv_bytecode.is_null() && k.spirv_word_count > 0 {
                        std::slice::from_raw_parts(k.spirv_bytecode, k.spirv_word_count).to_vec()
                    } else {
                        Vec::new()
                    };

                    compiled_nams.push(CompiledNam {
                        nam_name: name,
                        spirv_bytecode: bytecode,
                        stack_size_per_thread: k.stack_size_per_thread,
                    });
                }
            }

            (backend.free_result)(raw_res);
            Ok(compiled_nams)
        }
    }
}

fn c_ptr_to_string(ptr: *mut c_char, default: &str) -> String {
    if ptr.is_null() {
        default.to_string()
    } else {
        unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}
