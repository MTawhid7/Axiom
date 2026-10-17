//! libsandbox: entering a Seatbelt profile.
//!
//! `sandbox_init` is declared in `sandbox.h`, deprecated since macOS 10.8.
//! `sandbox_init_with_parameters` and `sandbox_check` are exported by
//! `libsystem_sandbox` but declared in no SDK header. `sandbox_compile_string`,
//! `sandbox_apply` and the parameter functions are exported by `libsandbox.1`.
//! The signatures follow Chromium's `sandbox/mac/seatbelt.cc`.

use core::ffi::{CStr, c_char, c_int};
use core::marker::{PhantomData, PhantomPinned};
use core::ptr;
use std::ffi::CString;

use crate::profile::Method;

/// `SANDBOX_NAMED` (`sandbox.h`): the profile argument names a built-in
/// profile.
const SANDBOX_NAMED: u64 = 0x0001;

/// libsandbox's opaque parameter set.
#[repr(C)]
struct Params {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// A compiled profile, `sandbox_profile_t` as Chromium declares it in
/// `sandbox/mac/seatbelt.cc` (X11).
#[repr(C)]
struct Compiled {
    builtin: *mut c_char,
    data: *const u8,
    size: usize,
}

// libsystem_sandbox, part of libSystem.
unsafe extern "C" {
    fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *mut c_char) -> c_int;
    fn sandbox_init_with_parameters(
        profile: *const c_char,
        flags: u64,
        parameters: *const *const c_char,
        errorbuf: *mut *mut c_char,
    ) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
    fn sandbox_check(pid: c_int, operation: *const c_char, filter_type: c_int, ...) -> c_int;
    static kSBXProfilePureComputation: c_char;
}

#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_create_params() -> *mut Params;
    fn sandbox_set_param(params: *mut Params, key: *const c_char, value: *const c_char) -> c_int;
    fn sandbox_free_params(params: *mut Params);
    fn sandbox_compile_string(
        profile: *const c_char,
        params: *mut Params,
        errorbuf: *mut *mut c_char,
    ) -> *mut Compiled;
    fn sandbox_apply(profile: *mut Compiled) -> c_int;
    fn sandbox_free_profile(profile: *mut Compiled);
}

/// Enter the SBPL `profile`, with `params` as `(key, value)` pairs, by
/// `method`. On failure, returns libsandbox's error text.
pub fn enter(profile: &str, params: &[(&str, &str)], method: Method) -> Result<(), String> {
    let profile = c_string(profile)?;
    let mut owned = Vec::with_capacity(params.len() * 2);
    for &(key, value) in params {
        owned.push(c_string(key)?);
        owned.push(c_string(value)?);
    }
    match method {
        Method::InitWithParameters => init_with_parameters(&profile, &owned),
        Method::CompileAndApply => compile_and_apply(&profile, &owned),
        Method::Precompiled => {
            Err("precompiled profiles arrive as bytecode (apply_bytecode)".to_owned())
        }
    }
}

fn init_with_parameters(profile: &CStr, params: &[CString]) -> Result<(), String> {
    let mut pointers: Vec<*const c_char> = params.iter().map(|p| p.as_ptr()).collect();
    pointers.push(ptr::null());
    let mut error: *mut c_char = ptr::null_mut();
    // SAFETY: X1 (the profile and every parameter are live CStrings, and
    // `pointers` is null-terminated as the function expects); X2 (`error`).
    let status = unsafe {
        sandbox_init_with_parameters(profile.as_ptr(), 0, pointers.as_ptr(), &raw mut error)
    };
    if status == 0 {
        Ok(())
    } else {
        Err(take_error(error))
    }
}

fn compile_and_apply(profile: &CStr, params: &[CString]) -> Result<(), String> {
    compile_c(profile, params)?.apply()
}

/// Compile the SBPL `profile` with `params`, without applying it.
pub fn compile(profile: &str, params: &[(&str, &str)]) -> Result<CompiledProfile, String> {
    let profile = c_string(profile)?;
    let mut owned = Vec::with_capacity(params.len() * 2);
    for &(key, value) in params {
        owned.push(c_string(key)?);
        owned.push(c_string(value)?);
    }
    compile_c(&profile, &owned)
}

fn compile_c(profile: &CStr, params: &[CString]) -> Result<CompiledProfile, String> {
    let set = ParamSet::new()?;
    for pair in params.chunks_exact(2) {
        // SAFETY: X1; X3 (`set` owns a live parameter set).
        let status = unsafe { sandbox_set_param(set.0, pair[0].as_ptr(), pair[1].as_ptr()) };
        if status != 0 {
            return Err(format!("sandbox_set_param failed with {status}"));
        }
    }
    let mut error: *mut c_char = ptr::null_mut();
    // SAFETY: X1; X2 (`error`); X3 (`set` is live, and its strings outlive
    // the compilation because `params` does).
    let compiled = unsafe { sandbox_compile_string(profile.as_ptr(), set.0, &raw mut error) };
    if compiled.is_null() {
        Err(take_error(error))
    } else {
        Ok(CompiledProfile(compiled))
    }
}

/// Is this process sandboxed? (`sandbox_check` with no operation, as
/// Chromium's `Seatbelt::IsSandboxed` asks.)
pub fn is_sandboxed() -> bool {
    let pid = c_int::try_from(std::process::id()).expect("pids fit in an int");
    // SAFETY: a null operation and filter type 0 (`SANDBOX_FILTER_NONE`) take
    // no variadic arguments.
    unsafe { sandbox_check(pid, ptr::null(), 0) != 0 }
}

/// Apply the built-in "pure computation" profile with `sandbox_init`. Its man
/// page in SDK 27.0 says that processes built against that SDK "will be
/// killed on attempt to do so"; this exists to observe that.
pub fn enter_named_pure_computation() -> Result<(), String> {
    let name = &raw const kSBXProfilePureComputation;
    let mut error: *mut c_char = ptr::null_mut();
    // SAFETY: X8 (`name` points at libsandbox's NUL-terminated profile name);
    // X2 (`error`).
    let status = unsafe { sandbox_init(name, SANDBOX_NAMED, &raw mut error) };
    if status == 0 {
        Ok(())
    } else {
        Err(take_error(error))
    }
}

/// Apply compiled bytecode produced by [`CompiledProfile::bytecode`],
/// possibly in another process. The calling process only applies the
/// profile; it doesn't load libsandbox's compiler or pay for compilation.
pub fn apply_bytecode(bytecode: &[u8]) -> Result<(), String> {
    let mut profile = Compiled {
        builtin: ptr::null_mut(),
        data: bytecode.as_ptr(),
        size: bytecode.len(),
    };
    // SAFETY: X11. A stack `sandbox_profile_t` built as Chromium's
    // `ApplyCompiledProfile` builds it (no builtin name, borrowed bytes);
    // `bytecode` outlives the call, and libsandbox doesn't keep the pointer.
    let status = unsafe { sandbox_apply(&raw mut profile) };
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "sandbox_apply failed with {status}: {}",
            std::io::Error::last_os_error()
        ))
    }
}

/// Copy and free an error string from libsandbox.
fn take_error(error: *mut c_char) -> String {
    if error.is_null() {
        return "libsandbox reported an error without a message".to_owned();
    }
    // SAFETY: X3. libsandbox returned a NUL-terminated string that it owns
    // until `sandbox_free_error`, below.
    let text = unsafe { CStr::from_ptr(error) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: X3. Freed once, with the matching function, and not used again.
    unsafe { sandbox_free_error(error) };
    text
}

fn c_string(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| format!("{s:?} contains a NUL byte"))
}

/// Owns a libsandbox parameter set (X3).
struct ParamSet(*mut Params);

impl ParamSet {
    fn new() -> Result<Self, String> {
        // SAFETY: no arguments. The result is null or a new parameter set,
        // which `Self` then owns (X3).
        let raw = unsafe { sandbox_create_params() };
        if raw.is_null() {
            Err("sandbox_create_params failed".to_owned())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for ParamSet {
    fn drop(&mut self) {
        // SAFETY: X3. Freed once, by its only owner.
        unsafe { sandbox_free_params(self.0) };
    }
}

/// A compiled profile, freed on drop (X3).
pub struct CompiledProfile(*mut Compiled);

impl CompiledProfile {
    /// The compiled bytecode, for applying in another process with
    /// [`apply_bytecode`] (as Chromium's browser hands profiles to helpers).
    pub fn bytecode(&self) -> Result<Vec<u8>, String> {
        // SAFETY: X3 (`self` owns a live compiled profile); X11 (its layout).
        let compiled = unsafe { &*self.0 };
        if !compiled.builtin.is_null() || compiled.data.is_null() || compiled.size > 16 << 20 {
            return Err("the compiled profile has an unexpected shape".to_owned());
        }
        // SAFETY: X11. libsandbox owns `size` bytes at `data` for as long as
        // `self` lives; they are copied before this borrow ends.
        Ok(unsafe { core::slice::from_raw_parts(compiled.data, compiled.size) }.to_vec())
    }

    /// Apply this profile to the calling process.
    pub fn apply(&self) -> Result<(), String> {
        // SAFETY: X3 (`self` owns a live compiled profile).
        let status = unsafe { sandbox_apply(self.0) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!("sandbox_apply failed with {status}"))
        }
    }
}

impl std::fmt::Debug for CompiledProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompiledProfile")
    }
}

impl Drop for CompiledProfile {
    fn drop(&mut self) {
        // SAFETY: X3. Freed once, by its only owner, after `sandbox_apply`
        // has returned. Applying installs a copy in the kernel and leaves the
        // compiled profile with its owner; the probe tests run this path.
        unsafe { sandbox_free_profile(self.0) };
    }
}
