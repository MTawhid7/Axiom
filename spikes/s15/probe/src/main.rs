//! Spike S15's probe, for a stable local signing identity (the architecture
//! review's RV1-17).
//!
//! Two builds that differ only in the tag compiled in (`S15_BUILD_TAG`) stand
//! for a browser before and after a rebuild. `tools/s15-measure.sh` signs each
//! one, ad hoc or with S15's identity, puts it at the same path, and runs:
//! - `tag`: print the build's tag;
//! - `write KEYCHAIN SERVICE`: add a generic password to the keychain file at
//!   `KEYCHAIN`, as the app that creates it;
//! - `read KEYCHAIN SERVICE`: find that password.
//!
//! Both disallow the keychain's user interface first, so an access that would
//! have asked the user fails with `errSecInteractionNotAllowed` (-25308)
//! instead, and no dialog appears. The keychain is S15's own file under
//! `target/`, which the script unlocks; the login keychain is never opened.
#![deny(unsafe_code)]

mod platform;

use std::process::ExitCode;

use platform::Keychain;

/// The build's tag, so that two builds differ in their code and so in their
/// code-directory hashes.
const BUILD_TAG: &str = match option_env!("S15_BUILD_TAG") {
    Some(tag) => tag,
    None => "untagged",
};

const ACCOUNT: &str = "s15";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["tag"] => {
            println!("{BUILD_TAG}");
            ExitCode::SUCCESS
        }
        ["write", keychain, service] => {
            let status = with_keychain(keychain, |keychain| {
                keychain.add_password(
                    service,
                    ACCOUNT,
                    format!("s15-secret-{BUILD_TAG}").as_bytes(),
                )
            });
            println!("build={BUILD_TAG} op=write status={status}");
            exit(status)
        }
        ["read", keychain, service] => {
            let mut secret = String::new();
            let status = with_keychain(keychain, |keychain| {
                match keychain.find_password(service, ACCOUNT) {
                    Ok(bytes) => {
                        secret = String::from_utf8_lossy(&bytes).into_owned();
                        0
                    }
                    Err(status) => status,
                }
            });
            println!("build={BUILD_TAG} op=read status={status} secret={secret}");
            exit(status)
        }
        _ => {
            eprintln!("usage: s15-probe tag | write KEYCHAIN SERVICE | read KEYCHAIN SERVICE");
            ExitCode::from(2)
        }
    }
}

/// Disallows the keychain's interface, opens the keychain and runs `operation`.
fn with_keychain(
    path: &str,
    operation: impl FnOnce(&Keychain) -> platform::OsStatus,
) -> platform::OsStatus {
    let status = platform::disallow_interaction();
    if status != 0 {
        return status;
    }
    match Keychain::open(path) {
        Ok(keychain) => operation(&keychain),
        Err(status) => status,
    }
}

fn exit(status: platform::OsStatus) -> ExitCode {
    if status == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
