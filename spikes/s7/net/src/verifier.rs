//! rustls's certificate verifier for Axiom: the chain goes to Security's
//! trust evaluation (NW6), as in rustls-platform-verifier; the handshake's
//! signatures are checked by the crypto provider. Each evaluation is timed,
//! since it blocks the calling thread (the I/O thread, in NW1's design)
//! while `trustd` answers.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, Error, OtherError, SignatureScheme};

use crate::platform::trust::{self, Rejection};

// `SecBase.h`: the codes mapped to rustls's own certificate errors, so that
// rustls sends the matching alert.
const ERR_HOST_NAME_MISMATCH: isize = -67602;
const ERR_CERTIFICATE_EXPIRED: isize = -67818;
const ERR_CERTIFICATE_NOT_VALID_YET: isize = -67819;
const ERR_CERTIFICATE_REVOKED: isize = -67820;
const ERR_NOT_TRUSTED: isize = -67843;
const ERR_CREATE_CHAIN_FAILED: isize = -25318;

/// One trust evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Evaluation {
    pub(crate) host: String,
    pub(crate) took: Duration,
    pub(crate) certificates: usize,
    pub(crate) stapled_ocsp: bool,
    pub(crate) rejection: Option<Rejection>,
}

#[derive(Debug)]
pub(crate) struct SystemVerifier {
    provider: Arc<CryptoProvider>,
    evaluations: Mutex<Vec<Evaluation>>,
}

impl SystemVerifier {
    pub(crate) fn new(provider: Arc<CryptoProvider>) -> Self {
        Self {
            provider,
            evaluations: Mutex::new(Vec::new()),
        }
    }

    /// The evaluations so far, emptying the log.
    pub(crate) fn take(&self) -> Vec<Evaluation> {
        std::mem::take(
            &mut *self
                .evaluations
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }
}

impl ServerCertVerifier for SystemVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let name = server_name.to_str();
        let host = name.strip_suffix('.').unwrap_or(&name);
        let chain: Vec<&[u8]> = std::iter::once(end_entity.as_ref())
            .chain(intermediates.iter().map(AsRef::as_ref))
            .collect();
        let ocsp = (!ocsp_response.is_empty()).then_some(ocsp_response);
        let start = Instant::now();
        let result = trust::evaluate(&chain, host, ocsp, now.as_secs());
        let evaluation = Evaluation {
            host: host.to_owned(),
            took: start.elapsed(),
            certificates: chain.len(),
            stapled_ocsp: ocsp.is_some(),
            rejection: result.clone().err(),
        };
        self.evaluations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(evaluation);
        match result {
            Ok(()) => Ok(ServerCertVerified::assertion()),
            Err(rejection) => Err(Error::InvalidCertificate(certificate_error(rejection))),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn certificate_error(rejection: Rejection) -> CertificateError {
    match rejection.code {
        ERR_HOST_NAME_MISMATCH => CertificateError::NotValidForName,
        ERR_CERTIFICATE_EXPIRED => CertificateError::Expired,
        ERR_CERTIFICATE_NOT_VALID_YET => CertificateError::NotValidYet,
        ERR_CERTIFICATE_REVOKED => CertificateError::Revoked,
        ERR_NOT_TRUSTED | ERR_CREATE_CHAIN_FAILED => CertificateError::UnknownIssuer,
        _ => CertificateError::Other(OtherError(Arc::new(TrustError(rejection)))),
    }
}

/// Security's rejection, as an error rustls can carry.
#[derive(Debug)]
struct TrustError(Rejection);

impl std::fmt::Display for TrustError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.0.description, self.0.code)
    }
}

impl std::error::Error for TrustError {}
