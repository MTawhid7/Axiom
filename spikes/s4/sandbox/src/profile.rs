//! Seatbelt profiles (SBPL) and their parameters.
//!
//! Profiles live in `profiles/` and are compiled into the helper, so a helper
//! never reads its profile from disk after start-up.

use std::fmt;
use std::str::FromStr;

/// The renderer-like profile: deny everything except the broker's rendezvous
/// service, and deny dynamic code generation explicitly.
pub const RENDERER: &str = include_str!("../profiles/renderer.sb");
/// [`RENDERER`] plus dynamic code generation, for the JIT renderer variant.
pub const RENDERER_JIT: &str = include_str!("../profiles/renderer-jit.sb");
/// Default deny and the rendezvous only, with no rule about dynamic code: the
/// evidence that `(deny default)` alone doesn't stop `MAP_JIT` (S4-Q4).
pub const BARE: &str = include_str!("../profiles/bare.sb");
/// [`RENDERER`] plus what CoreText needs (S4-Q5).
pub const TEXT: &str = include_str!("../profiles/text.sb");

/// Name of the parameter that carries the broker's rendezvous service name.
pub const RENDEZVOUS_PARAM: &str = "RENDEZVOUS";

/// A profile a helper can apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// [`RENDERER`].
    Renderer,
    /// [`RENDERER_JIT`].
    RendererJit,
    /// [`BARE`].
    Bare,
    /// [`TEXT`].
    Text,
}

impl Profile {
    /// The profile's SBPL source.
    pub fn source(self) -> &'static str {
        match self {
            Self::Renderer => RENDERER,
            Self::RendererJit => RENDERER_JIT,
            Self::Bare => BARE,
            Self::Text => TEXT,
        }
    }

    /// The name used on command lines and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Renderer => "renderer",
            Self::RendererJit => "renderer-jit",
            Self::Bare => "bare",
            Self::Text => "text",
        }
    }
}

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "renderer" => Ok(Self::Renderer),
            "renderer-jit" => Ok(Self::RendererJit),
            "bare" => Ok(Self::Bare),
            "text" => Ok(Self::Text),
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
    /// `sandbox_compile_string`, then `sandbox_apply`, both in the helper.
    CompileAndApply,
    /// The broker compiles the profile and sends the bytecode over the
    /// inherited socket; the helper only calls `sandbox_apply`. This is how
    /// Chromium's browser hands profiles to its helpers.
    Precompiled,
}

impl Method {
    /// The name used on command lines and in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::InitWithParameters => "init",
            Self::CompileAndApply => "compile",
            Self::Precompiled => "precompiled",
        }
    }
}

impl FromStr for Method {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "init" => Ok(Self::InitWithParameters),
            "compile" => Ok(Self::CompileAndApply),
            "precompiled" => Ok(Self::Precompiled),
            other => Err(format!("unknown method {other:?}")),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InitWithParameters => "sandbox_init_with_parameters",
            Self::CompileAndApply => "sandbox_compile_string + sandbox_apply",
            Self::Precompiled => "sandbox_apply of bytecode compiled by the broker",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_and_methods_round_trip_through_their_names() {
        for profile in [
            Profile::Renderer,
            Profile::RendererJit,
            Profile::Bare,
            Profile::Text,
        ] {
            assert_eq!(profile.name().parse::<Profile>(), Ok(profile));
        }
        for method in [
            Method::InitWithParameters,
            Method::CompileAndApply,
            Method::Precompiled,
        ] {
            assert_eq!(method.name().parse::<Method>(), Ok(method));
        }
        assert!("none".parse::<Profile>().is_err());
    }

    /// A profile's rules, without its `;` comments.
    fn rules(source: &str) -> String {
        source
            .lines()
            .map(|line| line.split(';').next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_profile_denies_by_default_and_names_its_parameter() {
        for profile in [
            Profile::Renderer,
            Profile::RendererJit,
            Profile::Bare,
            Profile::Text,
        ] {
            let rules = rules(profile.source());
            assert!(rules.contains("(deny default)"), "{profile}");
            assert!(
                rules.contains(&format!("(param \"{RENDEZVOUS_PARAM}\")")),
                "{profile}"
            );
            assert!(!rules.contains("(allow default)"), "{profile}");
        }
        // Only the JIT variant may generate code (JIT rule J3). Every other
        // profile but the bare one denies it explicitly, because default deny
        // doesn't cover it (S4-Q4).
        assert!(rules(RENDERER_JIT).contains("(allow dynamic-code-generation)"));
        for source in [RENDERER, TEXT] {
            assert!(rules(source).contains("(deny dynamic-code-generation)"));
        }
        assert!(!rules(BARE).contains("dynamic-code-generation"));
    }
}
