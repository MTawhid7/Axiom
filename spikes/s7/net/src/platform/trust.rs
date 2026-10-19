//! Security's trust evaluation (`SecTrust.h`, `SecPolicy.h`,
//! `SecCertificate.h`): a server's certificate chain checked against the
//! system's trust store, the user's trust settings, Apple's CA constraints
//! and revocation (NW6, SG7). The calls follow rustls-platform-verifier's
//! macOS verifier: an SSL server policy for the host name, the chain with the
//! server's certificate first, the time rustls supplies, and the server's
//! stapled OCSP response, if any.

use core::ptr::{self, NonNull};

use objc2_core_foundation::{CFArray, CFData, CFDate, CFError, CFRetained, CFString, CFType};
use objc2_security::{SecCertificate, SecPolicy, SecTrust};

/// `kCFAbsoluteTimeIntervalSince1970` (`CFDate.h`): seconds from 1970 to
/// CoreFoundation's epoch, 2001-01-01.
const ABSOLUTE_TIME_SINCE_1970: f64 = 978_307_200.0;

// Security: `SecPolicy.h` (N2). Declared here, not through objc2-security,
// because the header's assumed-nonnull result was observed to be NULL inside
// a sandbox that denies Security's bundle (S7, 2026-10-19); objc2-security
// panics on that, and the spike reports it instead.
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecPolicyCreateSSL(server: u8, hostname: Option<&CFString>) -> Option<NonNull<SecPolicy>>;
}

/// Why a chain was rejected: Security's error code (`SecBase.h`, such as
/// `errSecCertificateExpired`, -67818) and its description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rejection {
    pub(crate) code: isize,
    pub(crate) description: String,
}

impl Rejection {
    fn new(code: isize, description: impl Into<String>) -> Self {
        Self {
            code,
            description: description.into(),
        }
    }
}

/// Evaluate `chain` (DER, the server's certificate first) for `host`, at
/// `now` (seconds since 1970), with `ocsp`, a stapled OCSP response.
pub(crate) fn evaluate(
    chain: &[&[u8]],
    host: &str,
    ocsp: Option<&[u8]>,
    now: u64,
) -> Result<(), Rejection> {
    let mut certificates = Vec::with_capacity(chain.len());
    for der in chain {
        let data = CFData::from_bytes(der);
        // SAFETY: N4. `data` is a live `CFData`; the result is owned.
        let certificate = unsafe { SecCertificate::with_data(None, &data) }
            .ok_or_else(|| Rejection::new(0, "a certificate in the chain is not DER X.509"))?;
        certificates.push(certificate);
    }
    let certificates = CFArray::from_retained_objects(&certificates);
    let name = CFString::from_str(host);
    // SAFETY: N4. `name` is a live `CFString`; `Boolean` is a byte (N2).
    let policy = unsafe { SecPolicyCreateSSL(1, Some(&name)) }
        .ok_or_else(|| Rejection::new(0, "SecPolicyCreateSSL returned NULL"))?;
    // SAFETY: N4. A Create function's result, owned from here on.
    let policy = unsafe { CFRetained::from_raw(policy) };

    let mut raw: *mut SecTrust = ptr::null_mut();
    let certificates: &CFType = &certificates;
    let policy: &CFType = &policy;
    // SAFETY: N4 (an array of `SecCertificate` and a `SecPolicy`, as the
    // function documents); N1 (`raw` is a live out-parameter).
    let status = unsafe {
        SecTrust::create_with_certificates(certificates, Some(policy), NonNull::from(&mut raw))
    };
    let raw = NonNull::new(raw).filter(|_| status == 0).ok_or_else(|| {
        Rejection::new(
            status.try_into().unwrap_or(0),
            "SecTrustCreateWithCertificates failed",
        )
    })?;
    // SAFETY: N4. A Create function's result, owned from here on.
    let trust = unsafe { CFRetained::from_raw(raw) };

    #[expect(
        clippy::cast_precision_loss,
        reason = "seconds since 1970 stay below 2^53"
    )]
    let at = now as f64 - ABSOLUTE_TIME_SINCE_1970;
    let verify_at =
        CFDate::new(None, at).ok_or_else(|| Rejection::new(0, "CFDateCreate failed"))?;
    // SAFETY: N4. `trust` and `verify_at` are live objects of the documented
    // types.
    let status = unsafe { trust.set_verify_date(&verify_at) };
    if status != 0 {
        return Err(Rejection::new(
            status.try_into().unwrap_or(0),
            "SecTrustSetVerifyDate failed",
        ));
    }
    if let Some(response) = ocsp.filter(|r| !r.is_empty()) {
        let stapled = CFData::from_bytes(response);
        let stapled: &CFType = &stapled;
        // SAFETY: N4. `SecTrust.h` accepts "a CFData object containing a
        // single DER-encoded OCSPResponse".
        let status = unsafe { trust.set_ocsp_response(Some(stapled)) };
        if status != 0 {
            return Err(Rejection::new(
                status.try_into().unwrap_or(0),
                "SecTrustSetOCSPResponse failed",
            ));
        }
    }

    let mut error: *mut CFError = ptr::null_mut();
    // SAFETY: N4 (`trust` is live); N1 (`error` is a live out-parameter).
    if unsafe { trust.evaluate_with_error(&raw mut error) } {
        return Ok(());
    }
    let Some(error) = NonNull::new(error) else {
        return Err(Rejection::new(
            0,
            "trust evaluation failed without an error",
        ));
    };
    // SAFETY: N4. The error is returned retained (`CF_RETURNS_RETAINED` in
    // `SecTrust.h`), owned from here on.
    let error = unsafe { CFRetained::from_raw(error) };
    let description = error
        .description()
        .map_or_else(String::new, |d| d.to_string());
    Err(Rejection::new(error.code(), description))
}
