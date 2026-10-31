//! Helpers for `build.rs`: angzarr's settings for prost types ([`wire`]),
//! and the generated domain layer for marked-up components
//! ([`markup_domain`]), adapters ([`adapters`]), descriptors
//! ([`descriptors`]) and component registration ([`register`]), written
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

use angzarr_embedded_diagnostic::{Arg, Diagnostic};

pub use angzarr_codegen::{ProtoInput, PythonApp, Translate};

/// This crate's en-US messages.
pub const EN_US: &str = include_str!("../locales/en-US/angzarr-build.ftl");

/// The prost types of `protos`, by package (`<package>.rs` in `OUT_DIR`),
/// with angzarr's settings: `prost::Name` on every message, ordered maps,
/// and angzarr's own types taken from angzarr-proto.
pub fn wire(protos: &[&str], includes: &[&str]) -> Result<(), BuildError> {
    wire_into(&out_dir()?, protos, includes)
}

/// The domain layer for marked-up components (angzarr-embedded-macros), by package
/// (`<package>.domain.rs` in `OUT_DIR`): the domain types, their wire
/// conversions and `DomainMessage` impls (see
/// `angzarr_codegen::markup_domain`). `wire` names each package's prost
/// types, which must be built with [`wire`]'s settings (bytes as
/// `Vec<u8>`, maps as `BTreeMap`): the layer refuses others where it is
/// compiled, with `angzarr-codegen-wire-bytes` or `angzarr-codegen-wire-map`.
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
pub fn register(protos: &[&str], includes: &[&str], remote: &[&str]) -> Result<(), BuildError> {
    register_into(&out_dir()?, protos, includes, remote)
}

/// [`register`], into `out`, the `remote` domains (served by a child
/// process) left to the host.
pub fn register_into(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
    remote: &[&str],
) -> Result<(), BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let source = angzarr_codegen::register(&bytes, protos, remote).map_err(BuildError::Codegen)?;
    let path = out.join("angzarr-register.rs");
    std::fs::write(&path, source).map_err(|e| io(&path, &e))
}

/// The generated adapters of every component in `protos`
/// (`angzarr-adapters.rs` in `OUT_DIR`), for the crate root to `include!`
/// as `pub mod adapters` (see `angzarr_codegen::adapters`).
pub fn adapters(protos: &[&str], includes: &[&str], remote: &[&str]) -> Result<(), BuildError> {
    adapters_into(&out_dir()?, protos, includes, remote)
}

/// [`adapters`], into `out`, the user's interfaces under the crate's `src`
/// (`CARGO_MANIFEST_DIR`): each warning ([`adapters_from`]) is printed as
/// a `cargo:warning`, its code and its English.
pub fn adapters_into(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
    remote: &[&str],
) -> Result<(), BuildError> {
    let src = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("src");
    for warning in adapters_from(out, &src, protos, includes, remote)? {
        println!("cargo:warning={}: {warning}", warning.code());
    }
    Ok(())
}

/// [`adapters`], into `out`, reading the user's interfaces under `src`: a
/// policy's declares the commands it translates the refusals of. A
/// policy's interface that is not there (not scaffolded yet, or moved) is
/// a [`BuildWarning::InterfaceMissing`], not silence: its adapter gets no
/// refusal callbacks, and Cargo reruns the script when the file appears.
pub fn adapters_from(
    out: &Path,
    src: &Path,
    protos: &[&str],
    includes: &[&str],
    remote: &[&str],
) -> Result<Vec<BuildWarning>, BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let missing = std::cell::RefCell::new(Vec::new());
    let read = |path: &str| {
        let full = src.join(path);
        println!("cargo:rerun-if-changed={}", full.display());
        let found = std::fs::read_to_string(&full).ok();
        if found.is_none() {
            missing.borrow_mut().push(BuildWarning::InterfaceMissing {
                path: full.display().to_string(),
            });
        }
        found
    };
    let source =
        angzarr_codegen::adapters(&bytes, protos, remote, &read).map_err(BuildError::Codegen)?;
    let path = out.join("angzarr-adapters.rs");
    std::fs::write(&path, source).map_err(|e| io(&path, &e))?;
    Ok(missing.into_inner())
}

/// What generation went on past, reported to the build (`cargo:warning`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BuildWarning {
    /// A policy's interface the adapters read for its refusal switch is not
    /// at `path`: its adapter gets no refusal callbacks.
    InterfaceMissing { path: String },
}

impl fmt::Display for BuildWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildWarning::InterfaceMissing { path } => write!(
                f,
                "{path}, a policy's interface, is not there: its adapter translates no refusal; scaffold it, or move it back"
            ),
        }
    }
}

impl Diagnostic for BuildWarning {
    fn code(&self) -> &'static str {
        match self {
            BuildWarning::InterfaceMissing { .. } => "angzarr-build-interface-missing",
        }
    }

    fn args(&self) -> Vec<(&'static str, Arg)> {
        match self {
            BuildWarning::InterfaceMissing { path } => vec![("path", path.as_str().into())],
        }
    }
}

/// Each component's generated `ComponentDescriptor` in `protos`
/// (`angzarr-descriptors.rs` in `OUT_DIR`, see
/// `angzarr_codegen::descriptors_rs`), the crate's version
/// (`CARGO_PKG_VERSION`) its rules version, the `remote` domains (served
/// by a child process) left to their process. Unversioned, the descriptors'
/// rules version is empty, which the coordinator refuses.
pub fn descriptors(protos: &[&str], includes: &[&str], remote: &[&str]) -> Result<(), BuildError> {
    descriptors_into(&out_dir()?, protos, includes, remote, &rules_version())
}

/// The build's version: the crate's.
fn rules_version() -> String {
    std::env::var("CARGO_PKG_VERSION").unwrap_or_default()
}

/// [`descriptors`], into `out`, with `rules_version`.
pub fn descriptors_into(
    out: &Path,
    protos: &[&str],
    includes: &[&str],
    remote: &[&str],
    rules_version: &str,
) -> Result<(), BuildError> {
    let bytes = build_descriptors(out, protos, includes)?;
    let source = angzarr_codegen::descriptors_rs(&bytes, protos, remote, rules_version)
        .map_err(BuildError::Codegen)?;
    let path = out.join("angzarr-descriptors.rs");
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
    /// A generated file (the CLI's, never the user's) that existed with
    /// other content, rewritten.
    Regenerated(PathBuf),
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
    let bytes = scaffold_descriptors(protos, includes)?;
    let files =
        angzarr_codegen::scaffold(&bytes, protos, translate).map_err(BuildError::Codegen)?;
    write_scaffold(src, files, existing)
}

/// The uv project of one Python domain (see
/// `angzarr_codegen::scaffold_python`) under `root`, as [`scaffold_into`]
/// writes a Rust scaffold: a user's file once, a generated file every time.
pub fn scaffold_python_into(
    root: &Path,
    protos: &[&str],
    includes: &[&str],
    app: &PythonApp,
    existing: Existing,
) -> Result<Vec<Scaffolded>, BuildError> {
    let bytes = scaffold_descriptors(protos, includes)?;
    let files =
        angzarr_codegen::scaffold_python(&bytes, protos, app).map_err(BuildError::Codegen)?;
    write_scaffold(root, files, existing)
}

/// protoc's descriptor set for a scaffold: `protos`, and every other proto
/// in their directories, so a component a scaffolded message names (a
/// policy, a projector) is found without being scaffolded.
fn scaffold_descriptors(protos: &[&str], includes: &[&str]) -> Result<Vec<u8>, BuildError> {
    let work = scratch_dir()?;
    let inputs = with_siblings(protos, includes);
    let inputs: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let bytes = protoc_set(&work, &inputs, includes);
    let _ = std::fs::remove_dir_all(&work);
    bytes
}

/// `protos`, then every other proto of their directories (under the first
/// include holding each), as import names: what protoc reads so a
/// component a message of `protos` names is declared.
fn with_siblings(protos: &[&str], includes: &[impl AsRef<Path>]) -> Vec<String> {
    let mut inputs: Vec<String> = protos.iter().map(|p| p.to_string()).collect();
    for proto in protos {
        let Some(include) = includes
            .iter()
            .map(AsRef::as_ref)
            .find(|i| i.join(proto).exists())
        else {
            continue;
        };
        let dir = Path::new(proto).parent().unwrap_or(Path::new(""));
        let Ok(entries) = std::fs::read_dir(include.join(dir)) else {
            continue;
        };
        let mut siblings: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".proto"))
            .map(|name| dir.join(name).to_string_lossy().replace('\\', "/"))
            .collect();
        siblings.sort();
        for sibling in siblings {
            if !inputs.contains(&sibling) {
                inputs.push(sibling);
            }
        }
    }
    inputs
}

/// Writes `files` under `root`: an absent file is created; an existing
/// user's file is kept, or completed with `Existing::Complete`; an existing
/// generated file is rewritten when it differs. Every file is decided
/// before any is written, so a refusal writes none.
fn write_scaffold(
    root: &Path,
    files: Vec<angzarr_codegen::ScaffoldFile>,
    existing: Existing,
) -> Result<Vec<Scaffolded>, BuildError> {
    let mut writes = Vec::new();
    let mut done = Vec::new();
    for file in files {
        let path = root.join(&file.path);
        if !path.exists() {
            writes.push((path.clone(), file.source));
            done.push(Scaffolded::Created(path));
            continue;
        }
        let mine = std::fs::read_to_string(&path).map_err(|e| io(&path, &e))?;
        if file.owner == angzarr_codegen::Owner::Generated {
            if mine == file.source {
                done.push(Scaffolded::Kept(path));
            } else {
                writes.push((path.clone(), file.source));
                done.push(Scaffolded::Regenerated(path));
            }
            continue;
        }
        if existing == Existing::Keep {
            done.push(Scaffolded::Kept(path));
            continue;
        }
        match angzarr_codegen::complete(&file, &mine).map_err(BuildError::Codegen)? {
            Some(completed) => {
                writes.push((path.clone(), completed));
                done.push(Scaffolded::Completed(path));
            }
            None => done.push(Scaffolded::Kept(path)),
        }
    }
    for (path, source) in writes {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io(parent, &e))?;
        }
        std::fs::write(&path, source).map_err(|e| io(&path, &e))?;
    }
    Ok(done)
}

/// The message types of a scaffolded Python project: `buf generate` run in
/// `root` (its `buf.gen.yaml`), then every generated module's imports
/// rerooted (`angzarr_codegen::reroot_python`): angzarr's own under
/// `angzarr_client.proto`, the project's under its `_gen`.
pub fn proto_gen_python(root: &Path, app: &PythonApp) -> Result<(), BuildError> {
    run(root, "buf", &["generate"], |detail| BuildError::Buf {
        detail,
    })?;
    let gen_dir = root
        .join("src")
        .join(format!("angzarr_{}", app.domain.replace('-', "_")))
        .join("_gen");
    let mut modules = Vec::new();
    collect_python(&gen_dir, &mut modules)?;
    // The packages buf generated: the directories holding a `_pb2` module.
    let mut packages: Vec<String> = modules
        .iter()
        .filter(|m| {
            m.file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with("_pb2.py"))
        })
        .filter_map(|m| m.parent()?.strip_prefix(&gen_dir).ok())
        .map(|dir| dir.to_string_lossy().replace(['/', '\\'], "."))
        .filter(|p| !p.is_empty())
        .collect();
    packages.sort();
    packages.dedup();
    let packages: Vec<&str> = packages.iter().map(String::as_str).collect();
    let rules = angzarr_codegen::python_reroots(app, &packages);
    for module in modules {
        let source = std::fs::read_to_string(&module).map_err(|e| io(&module, &e))?;
        let rerooted = angzarr_codegen::reroot_python(&source, &rules);
        if rerooted != source {
            std::fs::write(&module, rerooted).map_err(|e| io(&module, &e))?;
        }
    }
    Ok(())
}

/// `uv lock` in `root`: the `uv.lock` angzarr-embedded's `uv run --locked`
/// needs.
pub fn lock_python(root: &Path) -> Result<(), BuildError> {
    run(root, "uv", &["lock"], |detail| BuildError::Uv { detail })
}

/// Every `.py` and `.pyi` file under `dir`.
fn collect_python(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    let entries = std::fs::read_dir(dir).map_err(|e| io(dir, &e))?;
    for entry in entries {
        let path = entry.map_err(|e| io(dir, &e))?.path();
        if path.is_dir() {
            collect_python(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "py" || e == "pyi") {
            out.push(path);
        }
    }
    Ok(())
}

/// `program args` in `dir`, its stderr the failure's detail.
fn run(
    dir: &Path,
    program: &str,
    args: &[&str],
    fail: impl Fn(String) -> BuildError,
) -> Result<(), BuildError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| fail(e.to_string()))?;
    if !output.status.success() {
        return Err(fail(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
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
    let inputs = with_siblings(protos, &includes);
    let inputs: Vec<&str> = inputs.iter().map(String::as_str).collect();
    protoc(out, &inputs, &includes)
}

/// protoc's descriptor set for `protos`, written under `work`.
fn protoc_set(work: &Path, protos: &[&str], includes: &[&str]) -> Result<Vec<u8>, BuildError> {
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
    /// `buf generate` could not run, or failed.
    Buf { detail: String },
    /// `uv` could not run, or failed.
    Uv { detail: String },
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
            BuildError::Buf { detail } => write!(f, "buf generate failed: {detail}"),
            BuildError::Uv { detail } => write!(f, "uv failed: {detail}"),
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
            BuildError::Buf { .. } => "angzarr-build-buf",
            BuildError::Uv { .. } => "angzarr-build-uv",
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
            BuildError::Protoc { detail }
            | BuildError::Prost { detail }
            | BuildError::Buf { detail }
            | BuildError::Uv { detail } => {
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
        Box::new(BuildError::Buf { detail: s() }),
        Box::new(BuildError::Uv { detail: s() }),
        Box::new(BuildError::Codegen(
            angzarr_codegen::CodegenError::NoWirePath { package: s() },
        )),
        Box::new(BuildWarning::InterfaceMissing { path: s() }),
    ]
}
