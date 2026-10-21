//! Helpers for `build.rs`: angzarr's settings for prost types ([`wire`]),
//! and the generated domain layer ([`domain`]), written to `OUT_DIR`, never
//! into the source tree.
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

use annals_diagnostic::{Arg, Diagnostic};

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
    let includes = with_angzarr(out, includes)?;
    rerun_if_changed(protos, &includes);
    let set = out.join("angzarr-domain.descriptors");
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
    let bytes = std::fs::read(&set).map_err(|e| io(&set, &e))?;
    for generated in angzarr_codegen::domain(&bytes, protos, wire).map_err(BuildError::Codegen)? {
        let path = out.join(format!("{}.domain.rs", generated.package));
        std::fs::write(&path, generated.source).map_err(|e| io(&path, &e))?;
    }
    Ok(())
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
