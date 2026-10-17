//! Seatbelt profiles (SBPL) and their parameters.
//!
//! Profiles live in `profiles/` and are compiled into the helper, so a helper
//! never reads its profile from disk after start-up.

use std::fmt;
use std::str::FromStr;

/// The renderer-like profile: deny everything except the broker's rendezvous
/// service.
pub const RENDERER: &str = include_str!("../profiles/renderer.sb");

/// Name of the parameter that carries the broker's rendezvous service name.
pub const RENDEZVOUS_PARAM: &str = "RENDEZVOUS";

/// A profile a helper can apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// [`RENDERER`].
    Renderer,
}

impl Profile {
    /// The profile's SBPL source.
    pub fn source(self) -> &'static str {
        match self {
            Self::Renderer => RENDERER,
        }
    }

    /// The name used on command lines and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Renderer => "renderer",
        }
    }
}

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "renderer" => Ok(Self::Renderer),
            other => Err(format!("unknown profile {other:?}")),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How a helper enters its profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `sandbox_init_with_parameters`: one call with the SBPL source and a
    /// NUL-terminated array of key/value parameters (Firefox's way).
    InitWithParameters,
    /// `sandbox_compile_string`, then `sandbox_apply` (the way Chromium's
    /// helpers apply a compiled profile).
    CompileAndApply,
}

impl Method {
    /// The name used on command lines and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::InitWithParameters => "init",
            Self::CompileAndApply => "compile",
        }
    }
}

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "init" => Ok(Self::InitWithParameters),
            "compile" => Ok(Self::CompileAndApply),
            other => Err(format!("unknown method {other:?}")),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InitWithParameters => "sandbox_init_with_parameters",
            Self::CompileAndApply => "sandbox_compile_string + sandbox_apply",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_and_methods_round_trip_through_their_names() {
        assert_eq!("renderer".parse::<Profile>(), Ok(Profile::Renderer));
        for method in [Method::InitWithParameters, Method::CompileAndApply] {
            assert_eq!(method.name().parse::<Method>(), Ok(method));
        }
        assert!("none".parse::<Profile>().is_err());
    }

    #[test]
    fn the_renderer_profile_denies_by_default_and_names_its_parameter() {
        assert!(RENDERER.contains("(deny default)"));
        assert!(RENDERER.contains(&format!("(param \"{RENDEZVOUS_PARAM}\")")));
        assert!(!RENDERER.contains("(allow default)"));
    }
}
