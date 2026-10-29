//! The startup of an angzarr-embedded application: the host config
//! (`angzarr.toml`, data: every domain, its language, how a remote one is
//! launched, and the store), the thin `src/main.rs` that runs it with
//! angzarr-embedded-host's `run`, the driver it hands the running
//! application to (`src/driver.rs`), the `build.rs` generating the rest, and
//! the `Cargo.toml`. All are written once and are the user's.
//!
//! The config is angzarr-embedded-host's `HostConfig` in its TOML form;
//! [`lint_startup`] checks one against the protos.

use std::collections::BTreeSet;
use std::fmt;

use angzarr_embedded_diagnostic::{Arg, Diagnostic};
use serde::Deserialize;

use crate::error::CodegenError;
use crate::model::{self, Component, ComponentKind, MARKUP_KINDS};
use crate::rust::snake;
use crate::scaffold::{Owner, ScaffoldFile};

/// The language whose domains run in the application's own process.
pub const IN_PROCESS_LANGUAGE: &str = "rust";
/// The languages the host launches with a default command (its
/// `default_launch`); any other needs a `launch`.
pub const LAUNCHED_LANGUAGES: &[&str] = &["python"];

/// The angzarr-embedded commit a generated application depends on: its
/// runtime, markup, diagnostics and host.
pub const ANGZARR_EMBEDDED_REV: &str = "169b79b21cd977cccdc198351a73d2d51531f3e9";
/// Where [`ANGZARR_EMBEDDED_REV`] is fetched from.
pub const ANGZARR_EMBEDDED_GIT: &str = "https://github.com/angzarr-io/angzarr-embedded";

/// One application's startup.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StartupApp {
    /// The application's name, for the prose.
    pub name: String,
    /// The domains served by a Python child process; every other domain is
    /// Rust, in process.
    pub python: Vec<String>,
    /// The protos' include directories, as the build script names them.
    pub includes: Vec<String>,
    /// The build dependency on angzarr-build, as a TOML value: `{ git =
    /// "...", rev = "..." }` or `{ path = "..." }`.
    pub angzarr_build: String,
}

/// The startup of every domain declared in `files`: `angzarr.toml`,
/// `build.rs` and `src/main.rs`, paths relative to the crate root.
pub fn startup(
    descriptor_set: &[u8],
    files: &[&str],
    app: &StartupApp,
) -> Result<Vec<ScaffoldFile>, CodegenError> {
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let domains = domains(&components);
    for python in &app.python {
        if !domains.contains(python) {
            return Err(CodegenError::PythonNoDomain {
                domain: python.clone(),
            });
        }
    }
    let anchor = components
        .first()
        .map(|c| c.anchor.full_name().to_string())
        .unwrap_or_default();
    let file = |path: &str, source: String| ScaffoldFile {
        component: anchor.clone(),
        path: path.to_string(),
        source,
        owner: Owner::User,
    };
    Ok(vec![
        file("Cargo.toml", manifest(app)),
        file("angzarr.toml", config(app, &domains)),
        file("build.rs", build_rs(files, app)),
        file("src/driver.rs", driver_rs(app)),
        file("src/main.rs", main_rs(app, &components, &domains, files)?),
    ])
}

/// The domains of the aggregates and process managers, in declaration
/// order.
fn domains(components: &[Component]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in components {
        if c.kind != ComponentKind::Policy && !out.contains(&c.domain) {
            out.push(c.domain.clone());
        }
    }
    out
}

fn config(app: &StartupApp, domains: &[String]) -> String {
    let mut out = format!(
        "# angzarr-embedded's host configuration for {}: every domain, the\n\
         # language it is written in and, for one served by a child process, how it\n\
         # is launched (its language's default launch, then its entry; `launch`\n\
         # replaces both). Written once by `angzarr scaffold startup` and yours from\n\
         # then on; `angzarr lint startup` checks it against the protos.\n",
        app.name
    );
    for domain in domains {
        out.push_str(&format!("\n[[domain]]\nname = {domain:?}\n"));
        if app.python.contains(domain) {
            out.push_str(&format!(
                "language = \"python\"\ndir = {domain:?}\nentry = [\"angzarr-{}\"]\n",
                domain.replace('_', "-")
            ));
        } else {
            out.push_str(&format!("language = {IN_PROCESS_LANGUAGE:?}\n"));
        }
    }
    out.push_str(
        "\n[store]\n\
         # SQLite, created when absent, relative to the working directory;\n\
         # `in_memory = true` keeps nothing on disk.\n\
         path = \"angzarr.db\"\n",
    );
    out
}

fn manifest(app: &StartupApp) -> String {
    let dep = format!("{{ git = \"{ANGZARR_EMBEDDED_GIT}\", rev = \"{ANGZARR_EMBEDDED_REV}\" }}");
    format!(
        "# {name} on angzarr-embedded. Written once by `angzarr scaffold startup`\n\
         # and yours from then on.\n\n\
         [package]\n\
         name = {package:?}\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\n\
         # angzarr-embedded, its markup and its host, at the commit this angzarr CLI\n\
         # generates for; the generated domain layer needs prost.\n\
         [dependencies]\n\
         angzarr-embedded = {dep}\n\
         angzarr-embedded-diagnostic = {dep}\n\
         angzarr-embedded-host = {dep}\n\
         angzarr-embedded-macros = {dep}\n\
         prost = \"0.14\"\n\n\
         [build-dependencies]\n\
         angzarr-build = {}\n",
        app.angzarr_build,
        name = app.name,
        package = app
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            })
            .collect::<String>(),
    )
}

fn driver_rs(app: &StartupApp) -> String {
    format!(
        "//! What {} does once it is up. Written once by `angzarr scaffold\n\
         //! startup` and yours: send commands with `app.handle(book, &sampled)`, read\n\
         //! a stream with `app.events(&cover)`. The application is used, and dropped,\n\
         //! outside an async context.\n\n\
         use angzarr_embedded_host::App;\n\n\
         pub fn drive(app: &App) -> Result<(), Box<dyn std::error::Error>> {{\n    \
         let _ = app;\n    \
         todo!(\"send the application's commands\")\n}}\n",
        app.name
    )
}

fn build_rs(files: &[&str], app: &StartupApp) -> String {
    let list = |items: Vec<String>| -> String {
        let one = format!("&[{}]", items.join(", "));
        if one.len() + 26 <= 100 {
            return one;
        }
        let lines: String = items.iter().map(|i| format!("    {i},\n")).collect();
        format!("&[\n{lines}]")
    };
    let protos = list(files.iter().map(|f| format!("{f:?}")).collect());
    let includes = list(app.includes.iter().map(|i| format!("{i:?}")).collect());
    format!(
        "//! Generates the wire types, the domain layer, the adapters and\n\
         //! `register()` into OUT_DIR from the protos; a domain angzarr.toml serves\n\
         //! from a child process gets a contract, not an adapter. Written once by\n\
         //! `angzarr scaffold startup`.\n\n\
         const PROTOS: &[&str] = {protos};\n\
         const INCLUDES: &[&str] = {includes};\n\n\
         fn main() -> Result<(), angzarr_build::BuildError> {{\n    \
         angzarr_build::app(\"angzarr.toml\", PROTOS, INCLUDES)\n}}\n"
    )
}

/// The Rust components of the application: every aggregate and process
/// manager of a domain not served by a child, and every policy between two
/// of its domains.
pub(crate) fn rust_components<'a>(
    components: &'a [Component],
    domains: &[String],
    remote: &[String],
) -> Vec<&'a Component> {
    components
        .iter()
        .filter(|c| match c.kind {
            ComponentKind::Policy => {
                domains.contains(&c.input_domain) && domains.contains(&c.output_domain)
            }
            _ => !remote.contains(&c.domain),
        })
        .collect()
}

fn main_rs(
    app: &StartupApp,
    components: &[Component],
    domains: &[String],
    files: &[&str],
) -> Result<String, CodegenError> {
    let mut packages = BTreeSet::new();
    for c in components {
        packages.insert(c.anchor.package_name().to_string());
    }
    let package = match packages.len() {
        1 => packages.into_iter().next().unwrap_or_default(),
        _ => {
            return Err(CodegenError::StartupPackages {
                files: files.join(", "),
            });
        }
    };
    let mut modules: Vec<String> = rust_components(components, domains, &app.python)
        .iter()
        .flat_map(|c| {
            let stem = snake(&c.name);
            [format!("pub mod {stem};"), format!("pub mod {stem}_impl;")]
        })
        .chain(std::iter::once("pub mod driver;".to_string()))
        .collect();
    modules.sort();
    let include = |name: &str| format!("    include!(concat!(env!(\"OUT_DIR\"), \"/{name}\"));\n");
    Ok(format!(
        "//! {} on angzarr-embedded: the Rust domains in process, and each\n\
         //! domain angzarr.toml lists in another language launched as a child\n\
         //! process. Written once by `angzarr scaffold startup`; `angzarr scaffold\n\
         //! rust` writes the modules.\n\n\
         pub mod wire {{\n{}}}\n\
         pub mod domain {{\n{}}}\n\
         pub mod adapters {{\n{}}}\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/angzarr-register.rs\"));\n\n\
         {}\n\n\
         /// Runs the application: its remote domains launched and every domain\n\
         /// registered (`angzarr_embedded_host::run`), then its driver. Dropping the\n\
         /// application stops the children.\n\
         fn main() -> Result<(), Box<dyn std::error::Error>> {{\n    \
         let app = angzarr_embedded_host::run(\"angzarr.toml\", register)?;\n    \
         driver::drive(&app)\n}}\n",
        app.name,
        include(&format!("{package}.rs")),
        include(&format!("{package}.domain.rs")),
        include("angzarr-adapters.rs"),
        modules.join("\n"),
    ))
}

/// angzarr-embedded-host's `HostConfig`, as far as the lints read it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostConfig {
    #[serde(rename = "domain", default)]
    domains: Vec<DomainConfig>,
    #[serde(default)]
    #[allow(dead_code)]
    supervision: Option<toml::Table>,
    #[serde(default)]
    #[allow(dead_code)]
    store: Option<toml::Table>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainConfig {
    name: String,
    language: String,
    #[serde(default)]
    launch: Option<Vec<String>>,
    #[serde(default)]
    #[allow(dead_code)]
    entry: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    dir: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    env: std::collections::BTreeMap<String, String>,
}

/// The domains a host config serves from a child process: every domain
/// not in the in-process language, or given a `launch`.
pub fn remote_domains(config: &str) -> Result<Vec<String>, CodegenError> {
    let parsed: HostConfig = toml::from_str(config).map_err(|e| CodegenError::StartupConfig {
        detail: e.message().to_string(),
    })?;
    Ok(parsed
        .domains
        .into_iter()
        .filter(|d| d.language != IN_PROCESS_LANGUAGE || d.launch.is_some())
        .map(|d| d.name)
        .collect())
}

/// The proto packages declaring the components of `files`, in order.
pub fn component_packages(
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<Vec<String>, CodegenError> {
    let mut packages: Vec<String> = Vec::new();
    for c in model::components(descriptor_set, files, MARKUP_KINDS)? {
        let package = c.anchor.package_name().to_string();
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    Ok(packages)
}

/// What [`lint_startup`] finds wrong with a host config.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StartupLint {
    /// A domain in a language the host has no launch for, and no `launch`.
    UnknownLanguage { domain: String, language: String },
    /// A `launch` with no program.
    EmptyLaunch { domain: String },
    /// A domain no aggregate or process manager of the protos owns.
    UnknownDomain { domain: String },
    /// A domain listed twice.
    DuplicateDomain { domain: String },
    /// A domain of the protos the config does not list.
    UnlistedDomain { domain: String },
}

impl fmt::Display for StartupLint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartupLint::UnknownLanguage { domain, language } => write!(
                f,
                "the domain {domain} is in {language}, which has no default launch; give it a launch"
            ),
            StartupLint::EmptyLaunch { domain } => {
                write!(f, "the domain {domain}'s launch names no program")
            }
            StartupLint::UnknownDomain { domain } => write!(
                f,
                "the domain {domain} is owned by no aggregate or process manager of the protos"
            ),
            StartupLint::DuplicateDomain { domain } => {
                write!(f, "the domain {domain} is listed more than once")
            }
            StartupLint::UnlistedDomain { domain } => write!(
                f,
                "the protos declare the domain {domain}, which the config does not list"
            ),
        }
    }
}

impl Diagnostic for StartupLint {
    fn code(&self) -> &'static str {
        match self {
            StartupLint::UnknownLanguage { .. } => "angzarr-codegen-startup-unknown-language",
            StartupLint::EmptyLaunch { .. } => "angzarr-codegen-startup-empty-launch",
            StartupLint::UnknownDomain { .. } => "angzarr-codegen-startup-unknown-domain",
            StartupLint::DuplicateDomain { .. } => "angzarr-codegen-startup-duplicate-domain",
            StartupLint::UnlistedDomain { .. } => "angzarr-codegen-startup-unlisted-domain",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        match self {
            StartupLint::UnknownLanguage { domain, language } => vec![
                ("domain", domain.as_str().into()),
                ("language", language.as_str().into()),
            ],
            StartupLint::EmptyLaunch { domain }
            | StartupLint::UnknownDomain { domain }
            | StartupLint::DuplicateDomain { domain }
            | StartupLint::UnlistedDomain { domain } => vec![("domain", domain.as_str().into())],
        }
    }
}

/// One value of every lint, for the catalog test.
pub fn startup_lint_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    vec![
        Box::new(StartupLint::UnknownLanguage {
            domain: s(),
            language: s(),
        }),
        Box::new(StartupLint::EmptyLaunch { domain: s() }),
        Box::new(StartupLint::UnknownDomain { domain: s() }),
        Box::new(StartupLint::DuplicateDomain { domain: s() }),
        Box::new(StartupLint::UnlistedDomain { domain: s() }),
    ]
}

/// The lints of `config`, a host config's TOML, against the domains the
/// protos declare in `files`, in the config's order and then the protos'.
/// A config that does not read as a host config is refused.
pub fn lint_startup(
    config: &str,
    descriptor_set: &[u8],
    files: &[&str],
) -> Result<Vec<StartupLint>, CodegenError> {
    let parsed: HostConfig = toml::from_str(config).map_err(|e| CodegenError::StartupConfig {
        detail: e.message().to_string(),
    })?;
    let components = model::components(descriptor_set, files, MARKUP_KINDS)?;
    let declared = domains(&components);
    let mut lints = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for domain in &parsed.domains {
        let name = domain.name.clone();
        if seen.contains(&domain.name.as_str()) {
            lints.push(StartupLint::DuplicateDomain {
                domain: name.clone(),
            });
        }
        seen.push(&domain.name);
        if !declared.contains(&domain.name) {
            lints.push(StartupLint::UnknownDomain {
                domain: name.clone(),
            });
        }
        match &domain.launch {
            Some(launch) if launch.first().is_none_or(|p| p.trim().is_empty()) => {
                lints.push(StartupLint::EmptyLaunch { domain: name });
            }
            Some(_) => {}
            None => {
                let language = domain.language.as_str();
                if language != IN_PROCESS_LANGUAGE && !LAUNCHED_LANGUAGES.contains(&language) {
                    lints.push(StartupLint::UnknownLanguage {
                        domain: name,
                        language: domain.language.clone(),
                    });
                }
            }
        }
    }
    for domain in declared {
        if !seen.contains(&domain.as_str()) {
            lints.push(StartupLint::UnlistedDomain { domain });
        }
    }
    Ok(lints)
}
