//! Helpers for `build.rs`: angzarr's settings for prost types ([`wire`]),
//! and the generated domain layer ([`domain`], or [`markup_domain`] for
//! marked-up components) and component registration ([`register`]), written
//! to `OUT_DIR`, never into the source tree. And the one thing written into
//! the source tree, once, as the user's: the markup scaffold
//! ([`scaffold_into`], behind `angzarr scaffold rust`).
//!
//! These read the build environment themselves, so a calling build script
//! needs none of it: a crate whose lints forbid `std::env` in its own code
//! (a pure core) has a build script that is one call.
//!
//! Proto paths are import names, relative to one of `includes` (as a
//! `.proto` file's own `import` names them). angzarr's protos are always
//! importable: they come from angzarr-proto, never from a vendored copy.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use annals_diagnostic::{Arg, Diagnostic};

pub use angzarr_codegen::Translate;

/// This crate's en-US messages.
pub const EN_US: &str = include_str!("../locales/en-US/angzarr-build.ftl");

/// The prost types of `protos`, by package (`<package>.rs` in `OUT_DIR`),
/// with angzarr's settings: `prost::Name` on every message, ordered maps,
/// and angzarr's own types taken from angzarr-proto.
pub fn wire(protos: &[&str], includes: &[&str]) -> Result<(), BuildError> {
    wire_into(&out_dir()?, protos, includes)
}

/// The domain layer of every component in `protos`, by package
/// (`<package>.domain.rs` in `OUT_DIR`). `wire` maps each package to the Rust
/// path of its prost types (see `angzarr_codegen::domain`).
pub fn domain(protos: &[&str], includes: &[&str], wire: &[(&str, &str)]) -> Result<(), BuildError> {
    domain_into(&out_dir()?, protos, includes, wire)
}

/// The domain layer for marked-up components (annals-macros), by package
/// (`<package>.domain.rs` in `OUT_DIR`): the domain types, their wire
/// conversions and `DomainMessage` impls (see
/// `angzarr_codegen::markup_domain`).
pub fn markup_domain(
    protos: &[&str],
    includes: &[&str],
    wire: &[(&str, &str)],
) -> Result<(), BuildError> {
    markup_domain_into(&out_dir()?, protos, includes, wire)
}

/// [`markup_domain`], into `out`.
pub fn markup_domain_into(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
    wire: &[(&str, &str)],
) -> Result<(), BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let generated =
        angzarr_codegen::markup_domain(&bytes, protos, wire).map_err(BuildError::Codegen)?;
    write_domain(out, generated)
}

/// The generated `register(&mut Registry)` for every component in
/// `protos` (`angzarr-register.rs` in `OUT_DIR`), for the crate root to
/// `include!`.
pub fn register(protos: &[&str], includes: &[&str]) -> Result<(), BuildError> {
    register_into(&out_dir()?, protos, includes)
}

/// [`register`], into `out`.
pub fn register_into(out: &Path, protos: &[&str], includes: &[&str]) -> Result<(), BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let source = angzarr_codegen::register(&bytes, protos).map_err(BuildError::Codegen)?;
    let path = out.join("angzarr-register.rs");
    std::fs::write(&path, source).map_err(|e| io(&path, &e))
}

/// What [`scaffold_into`] does with a component's file that exists.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Existing {
    /// Leave it as it is: the file is the user's.
    Keep,
    /// `--missing`: add the stubs it lacks, changing nothing else.
    Complete,
}

/// What [`scaffold_into`] did with one component's file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Scaffolded {
    /// It was absent, and is written.
    Created(PathBuf),
    /// It existed and lacked stubs, which are added.
    Completed(PathBuf),
    /// It existed and is unchanged.
    Kept(PathBuf),
}

/// The markup scaffold of every component in `protos`, one file each
/// under `src` (see `angzarr_codegen::scaffold`), `translate` being the
/// refusal switch's translate selections. An absent file is written; an
/// existing one is the user's, kept, or completed with `Existing::Complete`.
/// Every file is decided before any is written, so a refusal writes none.
pub fn scaffold_into(
    src: &Path,
    protos: &[&str],
    includes: &[&str],
    translate: &[Translate],
    existing: Existing,
) -> Result<Vec<Scaffolded>, BuildError> {
    let work = scratch_dir()?;
    let bytes = descriptors(&work, protos, includes);
    let _ = std::fs::remove_dir_all(&work);
    let files =
        angzarr_codegen::scaffold(&bytes?, protos, translate).map_err(BuildError::Codegen)?;
    let mut writes = Vec::new();
    let mut done = Vec::new();
    for file in files {
        let path = src.join(&file.path);
        if !path.exists() {
            writes.push((path.clone(), file.source));
            done.push(Scaffolded::Created(path));
            continue;
        }
        if existing == Existing::Keep {
            done.push(Scaffolded::Kept(path));
            continue;
        }
        let mine = std::fs::read_to_string(&path).map_err(|e| io(&path, &e))?;
        match angzarr_codegen::complete(&file, &mine).map_err(BuildError::Codegen)? {
            Some(completed) => {
                writes.push((path.clone(), completed));
                done.push(Scaffolded::Completed(path));
            }
            None => done.push(Scaffolded::Kept(path)),
        }
    }
    if !writes.is_empty() {
        std::fs::create_dir_all(src).map_err(|e| io(src, &e))?;
    }
    for (path, source) in writes {
        std::fs::write(&path, source).map_err(|e| io(&path, &e))?;
    }
    Ok(done)
}

/// A fresh directory for protoc's output when there is no `OUT_DIR`.
fn scratch_dir() -> Result<PathBuf, BuildError> {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-build-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| io(&dir, &e))?;
    Ok(dir)
}

/// [`wire`], into `out`.
pub fn wire_into(out: &Path, protos: &[&str], includes: &[&str]) -> Result<(), BuildError> {
    let includes = with_angzarr(out, includes)?;
    rerun_if_changed(protos, &includes);
    prost_build::Config::new()
        .enable_type_names()
        .btree_map(["."])
        .extern_path(".io.angzarr.v1", "::angzarr_proto::v1")
        .out_dir(out)
        .compile_protos(protos, &includes)
        .map_err(|e| BuildError::Prost {
            detail: e.to_string(),
        })
}

/// [`domain`], into `out`.
pub fn domain_into(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
    wire: &[(&str, &str)],
) -> Result<(), BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let generated = angzarr_codegen::domain(&bytes, protos, wire).map_err(BuildError::Codegen)?;
    write_domain(out, generated)
}

/// Each package's domain layer, as `<package>.domain.rs` in `out`.
fn write_domain(out: &Path, generated: Vec<angzarr_codegen::Generated>) -> Result<(), BuildError> {
    for generated in generated {
        let path = out.join(format!("{}.domain.rs", generated.package));
        std::fs::write(&path, generated.source).map_err(|e| io(&path, &e))?;
    }
    Ok(())
}

/// [`descriptors`] for a build script: Cargo reruns it when a proto
/// changes.
fn build_descriptors(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
) -> Result<Vec<u8>, BuildError> {
    let includes = with_angzarr(out, includes)?;
    rerun_if_changed(protos, &includes);
    protoc(out, protos, &includes)
}

/// protoc's descriptor set for `protos`, written under `work`.
fn descriptors(work: &Path, protos: &[&str], includes: &[&str]) -> Result<Vec<u8>, BuildError> {
    protoc(work, protos, &with_angzarr(work, includes)?)
}

/// protoc's descriptor set for `protos` (`--include_imports`, so angzarr's
/// options survive), written under `work`.
fn protoc(work: &Path, protos: &[&str], includes: &[PathBuf]) -> Result<Vec<u8>, BuildError> {
    let set = work.join("angzarr.descriptors");
    let output = Command::new(prost_build::protoc_from_env())
        .arg("--include_imports")
        .arg(format!("--descriptor_set_out={}", set.display()))
        .args(includes.iter().map(|i| format!("-I{}", i.display())))
        .args(protos)
        .output()
        .map_err(|e| BuildError::Protoc {
            detail: e.to_string(),
        })?;
    if !output.status.success() {
        return Err(BuildError::Protoc {
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    std::fs::read(&set).map_err(|e| io(&set, &e))
}

fn out_dir() -> Result<PathBuf, BuildError> {
    std::env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or(BuildError::NoOutDir)
}

/// `includes`, then angzarr's protos, written under `out`.
fn with_angzarr(out: &Path, includes: &[&str]) -> Result<Vec<PathBuf>, BuildError> {
    let dir = out.join("angzarr-proto");
    let angzarr = angzarr_proto::protos::write_include_dir(&dir).map_err(|e| io(&dir, &e))?;
    Ok(includes
        .iter()
        .map(PathBuf::from)
        .chain(std::iter::once(angzarr))
        .collect())
}

/// Cargo reruns the build script when a proto changes: each proto, found
/// under the first include holding it.
fn rerun_if_changed(protos: &[&str], includes: &[PathBuf]) {
    for proto in protos {
        if let Some(path) = includes.iter().map(|i| i.join(proto)).find(|p| p.exists()) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    println!("cargo:rerun-if-env-changed=PROTOC");
}

fn io(path: &Path, error: &std::io::Error) -> BuildError {
    BuildError::Io {
        path: path.display().to_string(),
        detail: error.to_string(),
    }
}

/// Why generation failed. A build script that returns it fails the build
/// with its English text.
#[derive(Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Not run by Cargo: `OUT_DIR` is unset.
    NoOutDir,
    /// Reading or writing a file under `OUT_DIR`.
    Io { path: String, detail: String },
    /// protoc could not run, or refused the protos.
    Protoc { detail: String },
    /// prost-build failed.
    Prost { detail: String },
    /// The generator refused the protos.
    Codegen(angzarr_codegen::CodegenError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::NoOutDir => f.write_str("OUT_DIR is unset: run this from a build script"),
            BuildError::Io { path, detail } => write!(f, "{path}: {detail}"),
            BuildError::Protoc { detail } => write!(f, "protoc failed: {detail}"),
            BuildError::Prost { detail } => write!(f, "prost-build failed: {detail}"),
            BuildError::Codegen(error) => error.fmt(f),
        }
    }
}

/// A build script's `main` returning `Result<(), BuildError>` prints the
/// error with `Debug`: the English text, not the variant.
impl fmt::Debug for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for BuildError {}

impl Diagnostic for BuildError {
    fn code(&self) -> &'static str {
        match self {
            BuildError::NoOutDir => "angzarr-build-no-out-dir",
            BuildError::Io { .. } => "angzarr-build-io",
            BuildError::Protoc { .. } => "angzarr-build-protoc",
            BuildError::Prost { .. } => "angzarr-build-prost",
            BuildError::Codegen(_) => "angzarr-build-codegen",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        match self {
            BuildError::NoOutDir => Vec::new(),
            BuildError::Io { path, detail } => vec![
                ("path", path.as_str().into()),
                ("detail", detail.as_str().into()),
            ],
            BuildError::Protoc { detail } | BuildError::Prost { detail } => {
                vec![("detail", detail.as_str().into())]
            }
            BuildError::Codegen(error) => vec![("error", Arg::nested(error))],
        }
    }
}

/// One value of every error this crate defines, for the catalog test.
pub fn diagnostic_samples() -> Vec<Box<dyn Diagnostic>> {
    let s = || "x".to_string();
    vec![
        Box::new(BuildError::NoOutDir),
        Box::new(BuildError::Io {
            path: s(),
            detail: s(),
        }),
        Box::new(BuildError::Protoc { detail: s() }),
        Box::new(BuildError::Prost { detail: s() }),
        Box::new(BuildError::Codegen(
            angzarr_codegen::CodegenError::NoWirePath { package: s() },
        )),
    ]
}
