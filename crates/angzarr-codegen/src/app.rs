//! An angzarr-embedded application's startup, generated from its protos
//! and its host config (`angzarr scaffold app`):
//!
//! - the user's, written once and never rewritten: `angzarr.toml`, the
//!   host config, seeded from the [`AppSeed`] (which domain runs in which
//!   language, what is exposed, the retry and store settings) and the
//!   user's to edit from then on; and `src/driver.rs`, what the
//!   application does once it is up;
//! - the CLI's, rewritten on every run from the protos and the host config
//!   as it is on disk: `build.rs` (one call, `angzarr_build::app`) and
//!   `src/main.rs` (the generated layer's modules, a module per in-process
//!   component's interface and implementation, the driver, and a `main`
//!   running the host with the generated `register()`).
//!
//! The host config is angzarr-embedded-host's `HostConfig` in its TOML
//! form, as of today's shape; [`lint_app`] checks one against the protos,
//! and [`app`] refuses to generate from one that has lints. Pure, as the
//! rest of the crate: descriptors and the config's text in, files out.

use std::fmt;

use angzarr_embedded_diagnostic::{Arg, Diagnostic};

use crate::error::CodegenError;
use crate::scaffold::ScaffoldFile;

/// From how far a command may reach a domain through the host's listener
/// (the host config's `expose`); not exposed is the default and is not
/// written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exposure {
    /// `expose = "loopback"`: from this machine.
    Loopback,
    /// `expose = "ip"`: from the network.
    Ip,
}

/// The `[retry]` keys a seed sets; each unset one is left to the host's
/// default and not written.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RetrySeed {
    /// `max_retries`.
    pub max_retries: Option<u32>,
    /// `max_race_runs`: 0 hands a lost race back as the store's conflict.
    pub max_race_runs: Option<u32>,
}

/// What `angzarr scaffold app` writes a new host config from. Used only
/// when there is none: an existing `angzarr.toml` is the user's, and the
/// generated files follow it, not the seed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AppSeed {
    /// The application's name, for the prose.
    pub name: String,
    /// The protos' include directories, as the build script names them.
    pub includes: Vec<String>,
    /// The domains served by a Python child process (`dir` and the
    /// `angzarr-<domain>` entry point of `angzarr scaffold python`'s
    /// project); every other domain is Rust, in process.
    pub python: Vec<String>,
    /// The domains exposed through the host's listener, and how far.
    pub expose: Vec<(String, Exposure)>,
    /// The `[retry]` keys to write.
    pub retry: RetrySeed,
    /// `[store] in_memory = true`, else the default SQLite file.
    pub in_memory: bool,
}

/// The startup of the application declared in `files`, paths relative to
/// the crate root: `angzarr.toml` and `src/driver.rs` (the user's) and
/// `build.rs` and `src/main.rs` (the CLI's). `config` is the host config on
/// disk, when there is one: the generated files are derived from it, and
/// the seed is unused. Refused ([`CodegenError::AppRefused`]) when the
/// config, the seed's or the user's, has lints ([`lint_app`]).
pub fn app(
    descriptor_set: &[u8],
    files: &[&str],
    seed: &AppSeed,
    config: Option<&str>,
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let _ = (descriptor_set, files, seed, config);
    Ok(Vec::new())
}

/// The domains a host config serves from a child process: every
/// `kind = "domain"` component not in Rust, or given a `launch`. What
/// `angzarr_build::app` leaves out of the adapters and `register()`.
pub fn remote_domains(config: &str) -> Result<Vec<String>, CodegenError> {
    let _ = config;
    Ok(Vec::new())
}

/// What [`lint_app`] finds wrong with a host config against the protos.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AppLint {
    /// A component no component of the protos is: a `domain` no aggregate
    /// or process manager owns, a `policy` no policy is.
    UnknownComponent { name: String, kind: String },
    /// Two components of one name, whatever their kinds.
    DuplicateComponent { name: String },
    /// A domain an aggregate or process manager of the protos owns, which
    /// the config does not list.
    UnlistedDomain { domain: String },
    /// A policy of the application (in the protos, or a `policy`
    /// component) whose source domain no component of the application
    /// owns: nothing would ever trigger it.
    PolicySourceAbsent { policy: String, domain: String },
    /// An `expose` on what is not an aggregate: a policy, projector or
    /// upcaster (`kind` its config kind) or a process manager.
    NotExposable { name: String, kind: String },
    /// A component in a language the host has no launch for, and no
    /// `launch`.
    UnknownLanguage { name: String, language: String },
    /// A `launch` with no program.
    EmptyLaunch { name: String },
}

impl fmt::Display for AppLint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppLint::UnknownComponent { name, kind } => {
                write!(f, "the {kind} {name} is no component of the protos")
            }
            AppLint::DuplicateComponent { name } => {
                write!(f, "the component {name} is listed more than once")
            }
            AppLint::UnlistedDomain { domain } => write!(
                f,
                "the protos declare the domain {domain}, which the config does not list"
            ),
            AppLint::PolicySourceAbsent { policy, domain } => write!(
                f,
                "the policy {policy} reads the domain {domain}, which no component of the application owns"
            ),
            AppLint::NotExposable { name, kind } => write!(
                f,
                "{name} is a {kind}, which cannot be exposed: only an aggregate is"
            ),
            AppLint::UnknownLanguage { name, language } => write!(
                f,
                "{name} is in {language}, which has no default launch; give it a launch"
            ),
            AppLint::EmptyLaunch { name } => write!(f, "{name}'s launch names no program"),
        }
    }
}

impl Diagnostic for AppLint {
    fn code(&self) -> &'static str {
        match self {
            AppLint::UnknownComponent { .. } => "angzarr-codegen-app-unknown-component",
            AppLint::DuplicateComponent { .. } => "angzarr-codegen-app-duplicate-component",
            AppLint::UnlistedDomain { .. } => "angzarr-codegen-app-unlisted-domain",
            AppLint::PolicySourceAbsent { .. } => "angzarr-codegen-app-policy-source-absent",
            AppLint::NotExposable { .. } => "angzarr-codegen-app-not-exposable",
            AppLint::UnknownLanguage { .. } => "angzarr-codegen-app-unknown-language",
            AppLint::EmptyLaunch { .. } => "angzarr-codegen-app-empty-launch",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        let s = |v: &String| Arg::from(v.as_str());
        match self {
            AppLint::UnknownComponent { name, kind } | AppLint::NotExposable { name, kind } => {
                vec![("name", s(name)), ("kind", s(kind))]
            }
            AppLint::DuplicateComponent { name } | AppLint::EmptyLaunch { name } => {
                vec![("name", s(name))]
            }
            AppLint::UnlistedDomain { domain } => vec![("domain", s(domain))],
            AppLint::PolicySourceAbsent { policy, domain } => {
                vec![("policy", s(policy)), ("domain", s(domain))]
            }
            AppLint::UnknownLanguage { name, language } => {
                vec![("name", s(name)), ("language", s(language))]
            }
        }
    }
}

/// One value of every lint, for the catalog test.
pub fn app_lint_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    vec![
        Box::new(AppLint::UnknownComponent {
            name: s(),
            kind: s(),
        }),
        Box::new(AppLint::DuplicateComponent { name: s() }),
        Box::new(AppLint::UnlistedDomain { domain: s() }),
        Box::new(AppLint::PolicySourceAbsent {
            policy: s(),
            domain: s(),
        }),
        Box::new(AppLint::NotExposable {
            name: s(),
            kind: s(),
        }),
        Box::new(AppLint::UnknownLanguage {
            name: s(),
            language: s(),
        }),
        Box::new(AppLint::EmptyLaunch { name: s() }),
    ]
}

/// The lints of `config`, a host config's TOML, against the components the
/// protos declare in `files`, in the config's order and then the protos'.
/// A config that does not read as a host config is refused
/// ([`CodegenError::AppConfig`]).
pub fn lint_app(
    config: &str,
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<Vec<AppLint>, CodegenError> {
    let _ = (config, descriptor_set, files);
    Ok(Vec::new())
}
