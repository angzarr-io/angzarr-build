//! An angzarr-embedded application's startup, generated from its protos
//! and its host config (`angzarr scaffold app`):
//!
//! - the user's, written once and never rewritten: `angzarr.toml`, the
//!   host config, seeded from the [`AppSeed`] (which domain runs in which
//!   language, what is exposed, the retry and store settings) and the
//!   user's to edit from then on; `Cargo.toml`, the crate's manifest, with
//!   angzarr-embedded pinned at [`ANGZARR_EMBEDDED_REV`]; and
//!   `src/driver.rs`, what the application does once it is up;
//! - the CLI's, rewritten on every run from the protos, the host config
//!   and the manifest as they are on disk: `build.rs` (one call,
//!   `angzarr_build::app`, or `app_without_domain` when the manifest names
//!   a [`DomainCrate`]) and `src/main.rs` (the generated layer's modules or
//!   the domain crate's, a module per in-process
//!   component's interface and implementation, the driver, and an async
//!   `main` running the host with the generated `register()`, which
//!   registers the in-process components the config lists).
//!
//! Every component is listed in the config, in-process policies and
//! projectors too, each by its name from the protos
//! ([`crate::component_name`]) and with its `kind`. The config is read as
//! angzarr-embedded-host's `HostConfig` reads it: an unknown key or value,
//! or a missing `kind`, is refused ([`CodegenError::AppConfig`]).
//! [`lint_app`] checks it against the protos, and [`app`] refuses to
//! generate from one that has lints. Pure, as the rest of the crate:
//! descriptors and the config's text in, files out.

use std::collections::BTreeMap;
use std::fmt;

use angzarr_embedded_diagnostic::{Arg, Diagnostic};
use serde::Deserialize;

use crate::common::snake;
use crate::error::CodegenError;
use crate::model::{self, Component, ComponentKind};
use crate::scaffold::{Owner, ScaffoldFile};

/// The angzarr-embedded commit a generated application's `Cargo.toml` pins
/// (its markup, diagnostics and host): the one this generator's output is
/// written against.
pub const ANGZARR_EMBEDDED_REV: &str = "c23457c14c857fc457324c563a12ca14fe4e7e79";
/// Where [`ANGZARR_EMBEDDED_REV`] is fetched from.
pub const ANGZARR_EMBEDDED_GIT: &str = "https://github.com/angzarr-io/angzarr-embedded";
/// Where angzarr-build is fetched from when a seed names no other source.
pub const ANGZARR_BUILD_GIT: &str = "https://github.com/angzarr-io/angzarr-cli";

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

/// What `angzarr scaffold app` writes a new host config and manifest from.
/// Used only when they do not exist: an existing `angzarr.toml` or
/// `Cargo.toml` is the user's, and the generated files follow it, not the
/// seed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AppSeed {
    /// The application's name, for the prose and, made a crate name, its
    /// package's.
    pub name: String,
    /// The protos' include directories, as the build script names them.
    pub includes: Vec<String>,
    /// The domains served by a Python child process (`dir` and the
    /// `angzarr-<domain>` entry point of `angzarr scaffold python`'s
    /// project); every other domain is Rust, in process.
    pub python: Vec<String>,
    /// The components exposed through the host's listener, by name, and
    /// how far.
    pub expose: Vec<(String, Exposure)>,
    /// The `[retry]` keys to write.
    pub retry: RetrySeed,
    /// `[store] in_memory = true`, else the default SQLite file.
    pub in_memory: bool,
    /// The manifest's angzarr-build build-dependency, a TOML value (`{ path
    /// = ".." }`); `None` is [`ANGZARR_BUILD_GIT`]'s default branch.
    pub angzarr_build: Option<String>,
    /// The library crate whose domain layer the application uses, written
    /// into the manifest (`[package.metadata.angzarr.domain]`); `None`, the
    /// application generates its own.
    pub domain: Option<DomainCrate>,
}

/// A domain layer another crate generates (its build script running
/// angzarr-build's `wire` and `markup_domain`), which an application uses
/// instead of generating its own: the manifest's
/// `[package.metadata.angzarr.domain]`, `crate` and `wire`. Its main.rs
/// names `<path>::domain` as `crate::domain` and `wire` as `crate::wire`,
/// and its build script generates the adapters, descriptors and
/// `register()` alone (`angzarr_build::app_without_domain`). The crate must
/// be a dependency of the application, and generated from the same protos.
#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainCrate {
    /// The crate's path, as Rust names it (`examples_proto`): its `domain`
    /// module is the domain layer.
    #[serde(rename = "crate")]
    pub path: String,
    /// Its wire types' module (`examples_proto::v1`), as the application's
    /// `crate::wire`; `None`, the application has no `crate::wire`.
    pub wire: Option<String>,
}

/// The startup of the application declared in `files`, paths relative to
/// the crate root: `angzarr.toml`, `Cargo.toml` and `src/driver.rs` (the
/// user's) and `build.rs` and `src/main.rs` (the CLI's). `config` and
/// `manifest` are the host config and the manifest on disk, when there are
/// ones: the generated files are derived from them (the manifest's
/// [`DomainCrate`]), and the seed's are unused. Refused
/// ([`CodegenError::AppRefused`]) when the config, the seed's or the
/// user's, has lints ([`lint_app`]); a manifest that does not read, or
/// whose domain crate is malformed, is [`CodegenError::AppManifest`], and a
/// domain crate's path that is no Rust path
/// [`CodegenError::AppDomainPath`].
pub fn app(
    descriptor_set: &[u8],
    files: &[&str],
    seed: &AppSeed,
    config: Option<&str>,
    manifest: Option<&str>,
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let components = model::components(descriptor_set, files, model::ALL_KINDS)?;
    let config_text = match config {
        Some(text) => text.to_string(),
        None => crate::headed("angzarr.toml", seeded_config(&components, seed)),
    };
    let lints = lint(&read(&config_text)?, &components);
    if !lints.is_empty() {
        return Err(CodegenError::AppRefused { lints });
    }
    let host = read(&config_text)?;
    let (manifest_text, domain) = match manifest {
        Some(text) => (text.to_string(), manifest_domain(text)?),
        None => (
            crate::headed("Cargo.toml", self::manifest(seed)),
            seed.domain.clone(),
        ),
    };
    if let Some(domain) = &domain {
        for path in std::iter::once(&domain.path).chain(&domain.wire) {
            if syn::parse_str::<syn::Path>(path).is_err() {
                return Err(CodegenError::AppDomainPath { path: path.clone() });
            }
        }
    }
    let user = |path: &str, source: String| ScaffoldFile {
        component: String::new(),
        path: path.to_string(),
        source,
        owner: Owner::User,
    };
    let generated = |path: &str, source: String| ScaffoldFile {
        component: String::new(),
        path: path.to_string(),
        source,
        owner: Owner::Generated,
    };
    // The host config and the manifest on disk, the user's, pass through
    // as they are; what is generated here carries the licence header.
    Ok(vec![
        user("angzarr.toml", config_text),
        user("Cargo.toml", manifest_text),
        generated(
            "build.rs",
            crate::headed(".rs", build_script(files, &seed.includes, domain.as_ref())),
        ),
        user("src/driver.rs", crate::headed(".rs", driver(&seed.name))),
        generated(
            "src/main.rs",
            crate::headed(
                ".rs",
                main_rs(&seed.name, &components, &host, domain.as_ref()),
            ),
        ),
    ])
}

/// The manifest's `[package.metadata.angzarr.domain]`, if it has one.
fn manifest_domain(text: &str) -> Result<Option<DomainCrate>, CodegenError> {
    #[derive(Deserialize)]
    struct Manifest {
        package: Option<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        metadata: Option<Metadata>,
    }
    #[derive(Deserialize)]
    struct Metadata {
        angzarr: Option<Angzarr>,
    }
    #[derive(Deserialize)]
    struct Angzarr {
        domain: Option<DomainCrate>,
    }
    let manifest: Manifest = toml::from_str(text).map_err(|e| CodegenError::AppManifest {
        detail: e.message().to_string(),
    })?;
    Ok(manifest
        .package
        .and_then(|p| p.metadata)
        .and_then(|m| m.angzarr)
        .and_then(|a| a.domain))
}

/// The domains a host config serves from a child process: every
/// `kind = "domain"` component not in Rust, or given a `launch`, by name.
/// What `angzarr_build::app` leaves out of the adapters and `register()`.
pub fn remote_domains(config: &str) -> Result<Vec<String>, CodegenError> {
    Ok(read(config)?
        .components
        .iter()
        .filter(|c| c.kind == Kind::Domain && !c.in_process())
        .map(|c| c.name.clone())
        .collect())
}

/// The components a host config runs in process, by name: every one in
/// Rust with no `launch`. What `angzarr_build::app`'s `register()`
/// registers, those the protos declare.
pub fn in_process_components(config: &str) -> Result<Vec<String>, CodegenError> {
    Ok(read(config)?
        .components
        .iter()
        .filter(|c| c.in_process())
        .map(|c| c.name.clone())
        .collect())
}

/// What [`lint_app`] finds wrong with a host config against the protos.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AppLint {
    /// A component no component of the protos is: a `domain` no aggregate
    /// or process manager is named, a `policy` no policy is.
    UnknownComponent { name: String, kind: String },
    /// Two components of one name, whatever their kinds.
    DuplicateComponent { name: String },
    /// A domain an aggregate or process manager of the protos owns, which
    /// the config does not list.
    UnlistedDomain { domain: String },
    /// A policy the config lists whose source domain no component of the
    /// application owns: nothing would ever trigger it.
    PolicySourceAbsent { policy: String, domain: String },
    /// A policy the config lists whose target domain no component of the
    /// application owns, in process or declared remote: its commands would
    /// go nowhere.
    PolicyTargetAbsent { policy: String, domain: String },
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
            AppLint::PolicyTargetAbsent { policy, domain } => write!(
                f,
                "the policy {policy} sends to the domain {domain}, which no component of the application owns; list it, remote if it runs elsewhere"
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
            AppLint::PolicyTargetAbsent { .. } => "angzarr-codegen-app-policy-target-absent",
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
            AppLint::PolicySourceAbsent { policy, domain }
            | AppLint::PolicyTargetAbsent { policy, domain } => {
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
        Box::new(AppLint::PolicyTargetAbsent {
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
    let components = model::components(descriptor_set, files, model::ALL_KINDS)?;
    Ok(lint(&read(config)?, &components))
}

// ------------------------------------------------------------ the config

/// angzarr-embedded-host's `HostConfig`, as strictly as it reads it: every
/// table refuses a key it does not know, a component's `kind` is
/// required, and `kind`, `expose` and `on_bug` are closed sets.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostConfig {
    #[serde(rename = "component", default)]
    components: Vec<ComponentConfig>,
    #[serde(default)]
    #[allow(dead_code)]
    supervision: Supervision,
    #[serde(default)]
    #[allow(dead_code)]
    store: Store,
    #[serde(default)]
    #[allow(dead_code)]
    policy: Policy,
    #[serde(default)]
    #[allow(dead_code)]
    retry: Retry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct ComponentConfig {
    name: String,
    language: String,
    kind: Kind,
    upcasts: Option<String>,
    launch: Option<Vec<String>>,
    #[serde(default)]
    entry: Vec<String>,
    dir: Option<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    expose: Expose,
}

impl ComponentConfig {
    /// In the application's process: in Rust, with no `launch`.
    fn in_process(&self) -> bool {
        self.launch.is_none() && self.language == "rust"
    }
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Domain,
    Policy,
    Projector,
    Upcaster,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Domain => "domain",
            Kind::Policy => "policy",
            Kind::Projector => "projector",
            Kind::Upcaster => "upcaster",
        }
    }
}

#[derive(Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Expose {
    #[default]
    None,
    Loopback,
    Ip,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
#[allow(dead_code)]
struct Supervision {
    handshake_timeout_ms: u64,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    shutdown_grace_ms: u64,
    terminate_grace_ms: u64,
    call_timeout_ms: u64,
    socket_dir: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
#[allow(dead_code)]
struct Store {
    path: String,
    in_memory: bool,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
#[allow(dead_code)]
struct Policy {
    on_bug: OnBug,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum OnBug {
    #[default]
    MovePast,
    Stay,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
#[allow(dead_code)]
struct Retry {
    max_retries: u32,
    max_race_runs: u32,
    initial_backoff_ms: u64,
    multiplier: f64,
    max_backoff_ms: u64,
    jitter: f64,
}

/// `text` read as a host config.
fn read(text: &str) -> Result<HostConfig, CodegenError> {
    toml::from_str(text).map_err(|e| CodegenError::AppConfig {
        detail: e.message().to_string(),
    })
}

/// A language the host launches with no `launch` given.
fn has_default_launch(language: &str) -> bool {
    language == "python"
}

/// The aggregate or process manager of `components` named `name`.
fn domain_named<'a>(components: &'a [Component], name: &str) -> Option<&'a Component> {
    components.iter().find(|c| {
        matches!(
            c.kind,
            ComponentKind::Aggregate | ComponentKind::ProcessManager
        ) && c.component_name == name
    })
}

/// The component of `kind` of `components` named `name`.
fn named<'a>(
    components: &'a [Component],
    kind: ComponentKind,
    name: &str,
) -> Option<&'a Component> {
    components
        .iter()
        .find(|c| c.kind == kind && c.component_name == name)
}

fn lint(config: &HostConfig, components: &[Component]) -> Vec<AppLint> {
    let mut lints = Vec::new();
    // The domains the application has: each listed domain's, by the
    // protos' component of its name, or its name.
    let domains: Vec<&str> = config
        .components
        .iter()
        .filter(|c| c.kind == Kind::Domain)
        .map(|c| domain_named(components, &c.name).map_or(c.name.as_str(), |d| d.domain.as_str()))
        .collect();
    for (i, listed) in config.components.iter().enumerate() {
        if config.components[..i].iter().any(|c| c.name == listed.name) {
            lints.push(AppLint::DuplicateComponent {
                name: listed.name.clone(),
            });
            continue;
        }
        let unknown = || AppLint::UnknownComponent {
            name: listed.name.clone(),
            kind: listed.kind.as_str().to_string(),
        };
        match listed.kind {
            Kind::Domain => match domain_named(components, &listed.name) {
                None => lints.push(unknown()),
                Some(pm)
                    if pm.kind == ComponentKind::ProcessManager
                        && listed.expose != Expose::None =>
                {
                    lints.push(AppLint::NotExposable {
                        name: listed.name.clone(),
                        kind: "process_manager".to_string(),
                    });
                }
                Some(_) => {}
            },
            Kind::Policy => match named(components, ComponentKind::Policy, &listed.name) {
                None => lints.push(unknown()),
                Some(policy) => {
                    if !domains.contains(&policy.input_domain.as_str()) {
                        lints.push(AppLint::PolicySourceAbsent {
                            policy: listed.name.clone(),
                            domain: policy.input_domain.clone(),
                        });
                    }
                    if !domains.contains(&policy.output_domain.as_str()) {
                        lints.push(AppLint::PolicyTargetAbsent {
                            policy: listed.name.clone(),
                            domain: policy.output_domain.clone(),
                        });
                    }
                }
            },
            // A projector or an upcaster may be written by hand: its name
            // is its own.
            Kind::Projector | Kind::Upcaster => {}
        }
        if listed.kind != Kind::Domain && listed.expose != Expose::None {
            lints.push(AppLint::NotExposable {
                name: listed.name.clone(),
                kind: listed.kind.as_str().to_string(),
            });
        }
        match &listed.launch {
            Some(launch) if launch.is_empty() => lints.push(AppLint::EmptyLaunch {
                name: listed.name.clone(),
            }),
            None if listed.language != "rust" && !has_default_launch(&listed.language) => lints
                .push(AppLint::UnknownLanguage {
                    name: listed.name.clone(),
                    language: listed.language.clone(),
                }),
            _ => {}
        }
    }
    for component in components.iter().filter(|c| {
        matches!(
            c.kind,
            ComponentKind::Aggregate | ComponentKind::ProcessManager
        )
    }) {
        let listed = config
            .components
            .iter()
            .any(|c| c.kind == Kind::Domain && c.name == component.component_name);
        if !listed {
            lints.push(AppLint::UnlistedDomain {
                domain: component.component_name.clone(),
            });
        }
    }
    lints
}

// ----------------------------------------------------------- the seed

/// The host config a seed makes for `components`: every aggregate and
/// process manager (Rust, or a Python child for a domain the seed names),
/// every policy whose two domains are among them and every projector all
/// of whose sources are among them (a Python child's too: the host records
/// its events), each in Rust; then `[retry]`
/// with the keys seeded, and `[store]`.
fn seeded_config(components: &[Component], seed: &AppSeed) -> String {
    struct Entry {
        name: String,
        kind: &'static str,
        language: &'static str,
        python_domain: Option<String>,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let domains: Vec<&Component> = components
        .iter()
        .filter(|c| {
            matches!(
                c.kind,
                ComponentKind::Aggregate | ComponentKind::ProcessManager
            )
        })
        .collect();
    for domain in &domains {
        let python = seed.python.contains(&domain.domain);
        entries.push(Entry {
            name: domain.component_name.clone(),
            kind: "domain",
            language: if python { "python" } else { "rust" },
            python_domain: python.then(|| domain.domain.clone()),
        });
    }
    // A Python domain the protos lack: listed, for the lint to name.
    for python in &seed.python {
        if !domains.iter().any(|d| &d.domain == python) {
            entries.push(Entry {
                name: python.clone(),
                kind: "domain",
                language: "python",
                python_domain: Some(python.clone()),
            });
        }
    }
    let has = |domain: &str| domains.iter().any(|d| d.domain == domain);
    for policy in components
        .iter()
        .filter(|c| c.kind == ComponentKind::Policy)
    {
        if has(&policy.input_domain) && has(&policy.output_domain) {
            entries.push(Entry {
                name: policy.component_name.clone(),
                kind: "policy",
                language: "rust",
                python_domain: None,
            });
        }
    }
    for projector in components
        .iter()
        .filter(|c| c.kind == ComponentKind::Projector)
    {
        // A Python child's events are recorded by the host, so a projector
        // of them runs in process as one of a Rust domain's does.
        if !projector.sources.is_empty() && projector.sources.iter().all(|s| has(s)) {
            entries.push(Entry {
                name: projector.component_name.clone(),
                kind: "projector",
                language: "rust",
                python_domain: None,
            });
        }
    }
    // An exposure naming nothing listed: listed, for the lint to name.
    for (name, _) in &seed.expose {
        if !entries.iter().any(|e| &e.name == name) {
            entries.push(Entry {
                name: name.clone(),
                kind: "domain",
                language: "rust",
                python_domain: None,
            });
        }
    }
    let mut out = format!(
        "# The host config of {}: every component, its kind, the language it\n\
         # is written in and, for one served by a child process, how it is\n\
         # launched. Written once by `angzarr scaffold app` and yours from then\n\
         # on: each run regenerates build.rs and src/main.rs from it, main.rs\n\
         # registering the in-process components it lists, and `angzarr lint\n\
         # app` checks it against the protos.\n",
        seed.name
    );
    for entry in &entries {
        out.push_str(&format!(
            "\n[[component]]\nname = \"{}\"\nkind = \"{}\"\nlanguage = \"{}\"\n",
            entry.name, entry.kind, entry.language
        ));
        if let Some(domain) = &entry.python_domain {
            out.push_str(&format!(
                "dir = \"{domain}\"\nentry = [\"angzarr-{domain}\"]\n"
            ));
        }
        if let Some((_, exposure)) = seed.expose.iter().find(|(n, _)| *n == entry.name) {
            let how = match exposure {
                Exposure::Loopback => "loopback",
                Exposure::Ip => "ip",
            };
            out.push_str(&format!("expose = \"{how}\"\n"));
        }
    }
    let retry: Vec<String> = [
        ("max_retries", seed.retry.max_retries),
        ("max_race_runs", seed.retry.max_race_runs),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|v| format!("{key} = {v}\n")))
    .collect();
    if !retry.is_empty() {
        out.push_str(&format!("\n[retry]\n{}", retry.concat()));
    }
    out.push_str(if seed.in_memory {
        "\n[store]\nin_memory = true\n"
    } else {
        "\n[store]\npath = \"angzarr.db\"\n"
    });
    out
}

/// `name` as a Cargo package name: ASCII letters and digits kept, lower
/// case, every other run of characters one `-`.
fn package_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// The seed's manifest: angzarr-embedded's crates at
/// [`ANGZARR_EMBEDDED_REV`], prost and tokio, and angzarr-build for the
/// build script.
fn manifest(seed: &AppSeed) -> String {
    let embedded =
        format!("{{ git = \"{ANGZARR_EMBEDDED_GIT}\", rev = \"{ANGZARR_EMBEDDED_REV}\" }}");
    let build = seed
        .angzarr_build
        .clone()
        .unwrap_or_else(|| format!("{{ git = \"{ANGZARR_BUILD_GIT}\" }}"));
    format!(
        "# The manifest of {name} on angzarr-embedded. Written once by `angzarr\n\
         # scaffold app` and yours from then on: add your logic's dependencies.\n\
         [package]\n\
         name = \"{package}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         {domain}\
         \n\
         [dependencies]\n\
         angzarr-embedded = {embedded}\n\
         angzarr-embedded-diagnostic = {embedded}\n\
         angzarr-embedded-host = {embedded}\n\
         angzarr-embedded-macros = {embedded}\n\
         prost = \"0.14\"\n\
         # The App is async, on the caller's runtime: tokio's.\n\
         tokio = {{ version = \"1\", features = [\"macros\", \"rt-multi-thread\"] }}\n\
         \n\
         [build-dependencies]\n\
         angzarr-build = {build}\n",
        name = seed.name,
        package = package_name(&seed.name),
        domain = seed.domain.as_ref().map(domain_table).unwrap_or_default(),
    )
}

/// The manifest's table naming `domain`, a blank line before it.
fn domain_table(domain: &DomainCrate) -> String {
    let wire = domain
        .wire
        .as_ref()
        .map(|w| format!("wire = {w:?}\n"))
        .unwrap_or_default();
    format!(
        "\n# The domain layer is {path}'s (its `domain` module), not generated\n\
         # here: add {path} to [dependencies].\n\
         [package.metadata.angzarr.domain]\n\
         crate = {path:?}\n{wire}",
        path = domain.path
    )
}

/// The generated build script: one call, `angzarr_build::app`, or over a
/// domain crate `angzarr_build::app_without_domain`.
fn build_script(files: &[&str], includes: &[String], domain: Option<&DomainCrate>) -> String {
    let list = |items: &[String]| -> String {
        let one = format!(
            "&[{}]",
            items
                .iter()
                .map(|i| format!("{i:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if one.len() <= 70 {
            return one;
        }
        let lines: String = items.iter().map(|i| format!("    {i:?},\n")).collect();
        format!("&[\n{lines}]")
    };
    let files: Vec<String> = files.iter().map(|f| f.to_string()).collect();
    let (doc, call) = match domain {
        None => (
            "//! The application's generated layer, into OUT_DIR: the wire types, the\n\
             //! markup domain layer, the adapters, the descriptors and `register()`,\n"
                .to_string(),
            "app",
        ),
        Some(domain) => (
            format!(
                "//! The application's generated layer, into OUT_DIR, over {}'s wire\n\
                 //! types and domain layer (Cargo.toml's [package.metadata.angzarr.domain]):\n\
                 //! the adapters, the descriptors and `register()`,\n",
                domain.path
            ),
            "app_without_domain",
        ),
    };
    format!(
        "// @generated by `angzarr scaffold app`. Do not edit: every run rewrites it.\n\
         \n\
         {doc}\
         //! from the protos and angzarr.toml; a domain angzarr.toml serves from a\n\
         //! child process is its command and event sets, and `register()`\n\
         //! registers the in-process components angzarr.toml lists.\n\
         \n\
         const PROTOS: &[&str] = {};\n\
         const INCLUDES: &[&str] = {};\n\
         \n\
         fn main() -> Result<(), angzarr_build::BuildError> {{\n    \
             angzarr_build::{call}(\"angzarr.toml\", PROTOS, INCLUDES)\n\
         }}\n",
        list(&files),
        list(includes)
    )
}

/// The user's driver, a stub.
fn driver(name: &str) -> String {
    format!(
        "//! What {name} does once it is up. Written once by `angzarr scaffold app`\n\
         //! and yours: send commands with `app.handle(book, &sampled).await`, read\n\
         //! a stream with `app.events(&cover).await`.\n\
         \n\
         use angzarr_embedded_host::App;\n\
         \n\
         pub async fn drive(app: &App) -> Result<(), Box<dyn std::error::Error>> {{\n    \
             let _ = app;\n    \
             todo!(\"send the application's commands\")\n\
         }}\n"
    )
}

/// The generated main: the generated layer's modules (over a domain crate,
/// its domain layer and wire types named in their place), a module per
/// in-process component's interface and implementation (each listed one
/// the protos declare), the driver, and an async `main`.
fn main_rs(
    name: &str,
    components: &[Component],
    config: &HostConfig,
    domain: Option<&DomainCrate>,
) -> String {
    let mut packages: Vec<&str> = components.iter().map(|c| c.anchor.package_name()).collect();
    packages.sort_unstable();
    packages.dedup();
    let include = |suffix: &str| -> String {
        packages
            .iter()
            .map(|p| format!("    include!(concat!(env!(\"OUT_DIR\"), \"/{p}{suffix}\"));\n"))
            .collect()
    };
    let mut modules: Vec<String> = vec!["driver".to_string()];
    for listed in config.components.iter().filter(|c| c.in_process()) {
        let component = match listed.kind {
            Kind::Domain => domain_named(components, &listed.name),
            Kind::Policy => named(components, ComponentKind::Policy, &listed.name),
            Kind::Projector => named(components, ComponentKind::Projector, &listed.name),
            Kind::Upcaster => None,
        };
        if let Some(component) = component {
            let stem = snake(&component.name);
            modules.push(format!("{stem}_impl"));
            modules.push(stem);
        }
    }
    modules.sort_unstable();
    modules.dedup();
    let modules: String = modules.iter().map(|m| format!("pub mod {m};\n")).collect();
    let layer = match domain {
        None => format!(
            "pub mod wire {{\n{}}}\npub mod domain {{\n{}}}\n",
            include(".rs"),
            include(".domain.rs")
        ),
        Some(domain) => {
            let wire = domain
                .wire
                .as_ref()
                .map(|w| format!("pub use {w} as wire;\n"))
                .unwrap_or_default();
            format!("pub use {}::domain;\n{wire}", domain.path)
        }
    };
    format!(
        "// @generated by `angzarr scaffold app` from angzarr.toml and the protos.\n\
         // Do not edit: every run rewrites it.\n\
         \n\
         //! {name} on angzarr-embedded: the Rust components in process, every\n\
         //! component angzarr.toml serves from a child process launched by the host,\n\
         //! the in-process ones it lists registered by the generated `register()`,\n\
         //! then the driver (`src/driver.rs`, yours).\n\
         \n\
         {layer}\
         pub mod adapters {{\n    include!(concat!(env!(\"OUT_DIR\"), \"/angzarr-adapters.rs\"));\n}}\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/angzarr-register.rs\"));\n\
         \n\
         {modules}\
         \n\
         /// Runs the application, then its driver, on tokio's runtime (the App is\n\
         /// async, on the caller's). Dropping the application stops the children.\n\
         #[tokio::main]\n\
         async fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    \
             let app = angzarr_embedded_host::run(\"angzarr.toml\", register).await?;\n    \
             driver::drive(&app).await\n\
         }}\n",
    )
}
