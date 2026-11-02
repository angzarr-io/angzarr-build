//! The protos a scenario declares, written out and compiled: each domain
//! its own package (`buy-in` is `buyin`), each message with the options the
//! scenario gives it, compiled by protoc into the descriptor set the
//! generator reads, with angzarr's options importable.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use crate::test_dir::TestDir;

/// A component's `ComponentOptions`.
#[derive(Clone, Debug, Default)]
pub struct Component {
    pub kind: String,
    pub domain: Option<String>,
    pub input_domain: Option<String>,
    pub output_domain: Option<String>,
}

/// A command's or fact's `CommandOptions`.
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub component: String,
    pub emits: Vec<String>,
    pub refuses: Vec<String>,
    pub fact: bool,
    pub starts: bool,
}

/// One message, with its options.
#[derive(Clone, Debug, Default)]
pub struct Message {
    pub component: Option<Component>,
    pub input: Option<Input>,
    /// Its `(event)` entries: the component, and the domain it names.
    pub events: Vec<(String, Option<String>)>,
    /// Its nested `Reason` enum's values, each with its `(refusal_status)`.
    pub reasons: Vec<(String, Option<String>)>,
}

/// A top-level enum's values, each with its `(refusal_status)`.
pub type Enum = Vec<(String, Option<String>)>;

/// Every message and enum a scenario declared, by full name.
#[derive(Clone, Debug, Default)]
pub struct Protos {
    pub messages: BTreeMap<String, Message>,
    pub enums: BTreeMap<String, Enum>,
}

/// A domain's package: its name with no `-`.
pub fn package(domain: &str) -> String {
    domain.replace('-', "")
}

/// A type as a scenario names it, `buy-in.RecordFundsHeld`, by its full
/// proto name, `buyin.RecordFundsHeld`.
pub fn full(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((domain, rest)) => format!("{}.{rest}", package(domain)),
        None => name.to_string(),
    }
}

fn list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|i| format!("\"{i}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

fn status(status: &Option<String>) -> String {
    status
        .as_ref()
        .map(|s| format!(" [(io.angzarr.v1.refusal_status) = {s}]"))
        .unwrap_or_default()
}

fn component_option(c: &Component) -> String {
    let mut out = format!("    kind: COMPONENT_KIND_{}\n", c.kind);
    for (key, value) in [
        ("domain", &c.domain),
        ("input_domain", &c.input_domain),
        ("output_domain", &c.output_domain),
    ] {
        if let Some(value) = value {
            let _ = writeln!(out, "    {key}: \"{value}\"");
        }
    }
    format!("  option (io.angzarr.v1.component) = {{\n{out}  }};\n")
}

fn input_option(i: &Input) -> String {
    let mut out = format!("    component: \"{}\"\n", i.component);
    if !i.emits.is_empty() {
        let _ = writeln!(out, "    emits: {}", list(&i.emits));
    }
    if !i.refuses.is_empty() {
        let _ = writeln!(out, "    refuses: {}", list(&i.refuses));
    }
    for (key, set) in [("fact", i.fact), ("starts", i.starts)] {
        if set {
            let _ = writeln!(out, "    {key}: true");
        }
    }
    format!("  option (io.angzarr.v1.command) = {{\n{out}  }};\n")
}

fn event_option((component, domain): &(String, Option<String>)) -> String {
    let domain = domain
        .as_ref()
        .map(|d| format!(" domain: \"{d}\""))
        .unwrap_or_default();
    format!("  option (io.angzarr.v1.event) = {{component: \"{component}\"{domain}}};\n")
}

fn reason_enum(values: &[(String, Option<String>)]) -> String {
    let mut out = String::from("  enum Reason {\n    REASON_UNSPECIFIED = 0");
    let zero = values.iter().find(|(v, _)| v == "REASON_UNSPECIFIED");
    let _ = writeln!(out, "{};", zero.map(|(_, s)| status(s)).unwrap_or_default());
    for (n, (value, marked)) in (1..).zip(values.iter().filter(|(v, _)| v != "REASON_UNSPECIFIED"))
    {
        let _ = writeln!(out, "    {value} = {n}{};", status(marked));
    }
    out + "  }\n  Reason reason = 1;\n"
}

fn message(name: &str, m: &Message) -> String {
    let mut out = format!("message {name} {{\n");
    out += &m
        .component
        .as_ref()
        .map(component_option)
        .unwrap_or_default();
    out += &m.input.as_ref().map(input_option).unwrap_or_default();
    for event in &m.events {
        out += &event_option(event);
    }
    if !m.reasons.is_empty() {
        out += &reason_enum(&m.reasons);
    }
    out + "}\n\n"
}

fn top_enum(name: &str, values: &Enum) -> String {
    let prefix = name.to_uppercase();
    let mut out = format!("enum {name} {{\n  {prefix}_UNSPECIFIED = 0;\n");
    for (n, (value, marked)) in (1..).zip(values) {
        let _ = writeln!(out, "  {value} = {n}{};", status(marked));
    }
    out + "}\n\n"
}

impl Protos {
    /// The message `name` (a full name), declared empty if it is not yet.
    pub fn message(&mut self, name: &str) -> &mut Message {
        self.messages.entry(name.to_string()).or_default()
    }

    /// Each package's file: its path and its text.
    pub fn files(&self) -> BTreeMap<String, String> {
        let mut files: BTreeMap<String, String> = BTreeMap::new();
        let mut text = |full: &str, body: String| {
            let (package, _) = full.rsplit_once('.').expect("a full name");
            let file = files.entry(format!("{package}.proto")).or_insert_with(|| {
                format!("syntax = \"proto3\";\npackage {package};\nimport \"io/angzarr/v1/options.proto\";\n\n")
            });
            file.push_str(&body);
        };
        for (full, m) in &self.messages {
            text(
                full,
                message(full.rsplit_once('.').expect("a full name").1, m),
            );
        }
        for (full, values) in &self.enums {
            text(
                full,
                top_enum(full.rsplit_once('.').expect("a full name").1, values),
            );
        }
        files
    }
}

/// protoc's descriptor set for `files` (path, text), with angzarr's options
/// importable: the spec's own protos first, then angzarr-proto's. They are
/// written to a [`TestDir`] of their own, removed when compiled.
pub fn compile(files: &BTreeMap<String, String>) -> Vec<u8> {
    let dir = TestDir::new("angzarr-codegen-spec");
    let protos = dir.join("protos");
    std::fs::create_dir_all(&protos).expect("a scratch directory");
    for (path, text) in files {
        std::fs::write(protos.join(path), text).expect("a proto written");
    }
    let angzarr =
        angzarr_proto::protos::write_include_dir(&dir.join("angzarr")).expect("angzarr's protos");
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../angzarr-project/proto");
    let out = dir.join("set.bin");
    let protoc = std::env::var("PROTOC").unwrap_or_else(|_| "protoc".to_string());
    let status = Command::new(protoc)
        .arg("--include_imports")
        .arg(format!("--descriptor_set_out={}", out.display()))
        .arg(format!("-I{}", protos.display()))
        .arg(format!("-I{}", spec.display()))
        .arg(format!("-I{}", angzarr.display()))
        .args(files.keys())
        .status()
        .expect("protoc on PATH");
    assert!(status.success(), "protoc failed on {files:#?}");
    std::fs::read(&out).expect("the descriptor set")
}
