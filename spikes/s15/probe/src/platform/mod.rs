//! The platform layer: Security.framework's legacy keychain functions, the
//! only foreign calls this spike makes, and the only place in the crate where
//! `unsafe` is allowed (`tools/unsafe-policy.txt`).
//!
//! Every declaration was checked against SDK 27.0's `SecKeychain.h` and
//! `SecKeychainItem.h`, where these functions are deprecated since macOS 10.10
//! but still declared, and exported by `Security.tbd`. Each `unsafe` block
//! cites one of these invariants:
//!
//! - **K1, C strings.** Every string passed to C is a [`CString`] that is
//!   still alive when the call returns, and every length passed with one is
//!   its length in bytes without the terminator.
//! - **K2, out-parameters.** Every out-pointer points to a live, properly
//!   aligned local of the declared type, read only after the call returns.
//!   Out-parameters the header marks nullable, and that this crate doesn't
//!   need, are null.
//! - **K3, retained keychains.** A `SecKeychainRef` from `SecKeychainOpen` is
//!   owned by exactly one [`Keychain`], whose `Drop` releases it once with
//!   `CFRelease`.
//! - **K4, returned data.** Password data from
//!   `SecKeychainFindGenericPassword` is `len` readable bytes, copied out at
//!   once, then freed exactly once with `SecKeychainItemFreeContent`.
//!
//! [`CString`]: std::ffi::CString
#![allow(unsafe_code)]

use std::ffi::{CString, c_char, c_void};

/// `OSStatus`.
pub(crate) type OsStatus = i32;

/// `errSecParam`, returned for arguments this crate can't pass to C.
const ERR_SEC_PARAM: OsStatus = -50;

type SecKeychainRef = *mut c_void;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecKeychainOpen(path_name: *const c_char, keychain: *mut SecKeychainRef) -> OsStatus;
    fn SecKeychainSetUserInteractionAllowed(state: u8) -> OsStatus;
    fn SecKeychainAddGenericPassword(
        keychain: SecKeychainRef,
        service_name_length: u32,
        service_name: *const c_char,
        account_name_length: u32,
        account_name: *const c_char,
        password_length: u32,
        password_data: *const c_void,
        item_ref: *mut *mut c_void,
    ) -> OsStatus;
    fn SecKeychainFindGenericPassword(
        keychain_or_array: *const c_void,
        service_name_length: u32,
        service_name: *const c_char,
        account_name_length: u32,
        account_name: *const c_char,
        password_length: *mut u32,
        password_data: *mut *mut c_void,
        item_ref: *mut *mut c_void,
    ) -> OsStatus;
    fn SecKeychainItemFreeContent(attr_list: *mut c_void, data: *mut c_void) -> OsStatus;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// A C string and its length, as the keychain functions take them.
fn c_string(text: &str) -> Result<(CString, u32), OsStatus> {
    let length = u32::try_from(text.len()).map_err(|_| ERR_SEC_PARAM)?;
    let string = CString::new(text).map_err(|_| ERR_SEC_PARAM)?;
    Ok((string, length))
}

/// Disallows the keychain's user interface for this process, so an access that
/// would have asked the user fails with `errSecInteractionNotAllowed` instead.
pub(crate) fn disallow_interaction() -> OsStatus {
    // SAFETY: no pointers; the call sets a process-wide flag.
    unsafe { SecKeychainSetUserInteractionAllowed(0) }
}

/// An open keychain file.
#[derive(Debug)]
pub(crate) struct Keychain(SecKeychainRef);

impl Keychain {
    /// Opens the keychain file at `path`.
    pub(crate) fn open(path: &str) -> Result<Self, OsStatus> {
        let (path, _) = c_string(path)?;
        let mut keychain: SecKeychainRef = std::ptr::null_mut();
        // SAFETY: K1, `path` outlives the call; K2, `keychain` is a live local.
        let status = unsafe { SecKeychainOpen(path.as_ptr(), &raw mut keychain) };
        if status != 0 || keychain.is_null() {
            return Err(status);
        }
        Ok(Self(keychain))
    }

    /// Adds a generic password, created by this process.
    pub(crate) fn add_password(&self, service: &str, account: &str, password: &[u8]) -> OsStatus {
        let Ok((service, service_length)) = c_string(service) else {
            return ERR_SEC_PARAM;
        };
        let Ok((account, account_length)) = c_string(account) else {
            return ERR_SEC_PARAM;
        };
        let Ok(password_length) = u32::try_from(password.len()) else {
            return ERR_SEC_PARAM;
        };
        // SAFETY: K1, `service` and `account` outlive the call, with their
        // lengths; `password` is a live slice of `password_length` bytes; K2,
        // the item out-parameter is null; K3, `self.0` is the keychain this
        // value owns.
        unsafe {
            SecKeychainAddGenericPassword(
                self.0,
                service_length,
                service.as_ptr(),
                account_length,
                account.as_ptr(),
                password_length,
                password.as_ptr().cast(),
                std::ptr::null_mut(),
            )
        }
    }

    /// Finds a generic password, and returns a copy of it.
    pub(crate) fn find_password(&self, service: &str, account: &str) -> Result<Vec<u8>, OsStatus> {
        let (service, service_length) = c_string(service)?;
        let (account, account_length) = c_string(account)?;
        let mut length: u32 = 0;
        let mut data: *mut c_void = std::ptr::null_mut();
        // SAFETY: K1, `service` and `account` outlive the call, with their
        // lengths; K2, `length` and `data` are live locals, and the item
        // out-parameter is null; K3, `self.0` is the keychain this value owns.
        let status = unsafe {
            SecKeychainFindGenericPassword(
                self.0.cast_const(),
                service_length,
                service.as_ptr(),
                account_length,
                account.as_ptr(),
                &raw mut length,
                &raw mut data,
                std::ptr::null_mut(),
            )
        };
        if status != 0 || data.is_null() {
            return Err(status);
        }
        // SAFETY: K4, `data` is `length` readable bytes that the call returned.
        let copy =
            unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length as usize) }.to_vec();
        // SAFETY: K4, `data` is freed once, here, and not used afterwards.
        let freed = unsafe { SecKeychainItemFreeContent(std::ptr::null_mut(), data) };
        if freed != 0 {
            return Err(freed);
        }
        Ok(copy)
    }
}

impl Drop for Keychain {
    fn drop(&mut self) {
        // SAFETY: K3, this value owns the reference, released once here.
        unsafe { CFRelease(self.0.cast_const()) }
    }
}
