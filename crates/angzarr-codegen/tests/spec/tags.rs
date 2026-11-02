//! The tags on the scenarios the runner does not run, each naming why: the
//! one record of where angzarr-cli and the codegen tier part.
//!
//! - `divergence_*`: angzarr-cli does otherwise, deliberately; the why is
//!   its reason.
//! - `unsupported_*`: angzarr-cli has no surface of the kind.
//! - `unbound*`: angzarr-cli may well satisfy it, but no step binding
//!   drives it yet.
//!
//! A tag names a scenario by its ID (`C-0517`) or one row of an outline's
//! examples, counted from 1 across its tables (`C-0517#2`).

use std::collections::BTreeMap;

use cucumber::feature::Ext as _;
use cucumber::gherkin::{Feature, GherkinEnv, Scenario};

/// Why a scenario is not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag {
    pub name: &'static str,
    pub why: &'static str,
}

/// angzarr-cli has no lint of the protos that reports warnings: a
/// declaration the generator accepts passes in silence (`angzarr lint` is
/// the host config's alone, `lint app`).
const NO_WARNINGS: Tag = Tag {
    name: "unsupported_lint_warnings",
    why: "the generator reports no warnings: what it accepts passes in silence",
};

/// A rule of the declarations the generator does not check: it
/// generates from them as they are.
const NO_DECLARATION_CHECK: Tag = Tag {
    name: "unsupported_declaration_check",
    why: "the generator does not check this rule of the declarations: it generates from them",
};

/// What a scenario's business binary does at run time, built from what
/// is generated: no binding builds and runs one.
const UNBOUND_BUILT: Tag = Tag {
    name: "unbound_built_binary",
    why: "a built binary's behaviour: no binding builds and runs the generated code",
};

/// What the generated handlers' code says, read as Rust: no binding reads
/// it yet beyond a handler's markup.
const UNBOUND_GENERATED: Tag = Tag {
    name: "unbound_generated_code",
    why: "the generated code's shape: no binding reads it yet",
};

/// Every tagged scenario, by ID or outline row, in the tier's file order.
const TAGGED: &[(&str, Tag)] = &[
    // component_options.feature
    ("C-0514", NO_WARNINGS),
    ("C-0515", NO_WARNINGS),
    ("C-0516", NO_WARNINGS),
    ("C-0604", NO_DECLARATION_CHECK),
    ("C-0517", NO_WARNINGS),
    ("C-0605", NO_DECLARATION_CHECK),
    ("C-0582", NO_DECLARATION_CHECK),
    ("C-0606", NO_DECLARATION_CHECK),
    ("C-0611", NO_WARNINGS),
    ("C-0612", UNBOUND_GENERATED),
    ("C-0623", UNBOUND_GENERATED),
    ("C-0625", UNBOUND_GENERATED),
    ("C-0745", UNBOUND_BUILT),
    ("C-0626", NO_DECLARATION_CHECK),
    ("C-0650", UNBOUND_GENERATED),
    // descriptor.feature
    ("C-0565", UNBOUND_BUILT),
];

/// The scenario's ID, its one `@C-NNNN` tag.
pub fn id(scenario: &Scenario) -> Option<&str> {
    let mut ids = scenario
        .tags
        .iter()
        .filter(|t| t.starts_with("C-"))
        .map(String::as_str);
    let id = ids.next();
    match ids.next() {
        None => id,
        Some(_) => None,
    }
}

/// The number, from 1 across its examples' tables, of the outline row
/// `scenario` was expanded from, if it was.
fn row(scenario: &Scenario) -> Option<usize> {
    let mut first = 1;
    for examples in &scenario.examples {
        let rows = examples
            .table
            .as_ref()
            .map_or(0, |t| t.rows.len().saturating_sub(1));
        // An expanded row sits at its table's line plus its index plus two.
        let offset = scenario
            .position
            .line
            .checked_sub(examples.position.line + 2);
        if let Some(index) = offset.filter(|i| *i < rows) {
            return Some(first + index);
        }
        first += rows;
    }
    None
}

/// The scenario's key: its ID, then `#` and its row for an outline's row.
pub fn key(scenario: &Scenario) -> Option<String> {
    let id = id(scenario)?;
    Some(match row(scenario) {
        Some(row) => format!("{id}#{row}"),
        None => id.to_string(),
    })
}

/// The tag on `scenario`, by its row's key, else its ID.
pub fn of(scenario: &Scenario) -> Option<Tag> {
    let (key, id) = (key(scenario)?, id(scenario)?);
    let tagged = |wanted: &str| TAGGED.iter().find(|(k, _)| *k == wanted).map(|(_, t)| *t);
    tagged(&key).or_else(|| tagged(id))
}

/// Every scenario of the tier, its outlines expanded, in file order.
fn scenarios(features: &str) -> Vec<Scenario> {
    let mut paths: Vec<_> = std::fs::read_dir(features)
        .expect("the tier")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "feature"))
        .collect();
    paths.sort();
    let parsed = paths.iter().map(|path| {
        let feature = Feature::parse_path(path, GherkinEnv::default())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        feature
            .expand_examples()
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    });
    parsed
        .flat_map(|f| {
            f.scenarios
                .into_iter()
                .chain(f.rules.into_iter().flat_map(|r| r.scenarios))
        })
        .collect()
}

/// What the run left out, by tag, and how many it ran.
pub fn listing(features: &str) -> String {
    let all = scenarios(features);
    let mut by_tag: BTreeMap<&str, (Tag, Vec<String>)> = BTreeMap::new();
    for scenario in &all {
        if let Some(tag) = of(scenario) {
            let entry = by_tag.entry(tag.name).or_insert((tag, Vec::new()));
            entry.1.push(key(scenario).unwrap_or_default());
        }
    }
    let tagged: usize = by_tag.values().map(|(_, keys)| keys.len()).sum();
    let mut out = format!(
        "codegen: {} scenarios (outline rows counted), {} bound, {} tagged\n",
        all.len(),
        all.len() - tagged,
        tagged
    );
    for (name, (tag, keys)) in by_tag {
        out += &format!(
            "  @{name} ({}): {}\n    {}\n",
            keys.len(),
            tag.why,
            keys.join(" ")
        );
    }
    out
}

/// Every scenario carries one ID; every tag names a scenario, or one of
/// its rows, once, and is of a kind.
pub fn check(features: &str) {
    let all = scenarios(features);
    for scenario in &all {
        assert!(
            id(scenario).is_some(),
            "{}: {:?}",
            scenario.name,
            scenario.tags
        );
    }
    let keys: Vec<String> = all.iter().filter_map(key).collect();
    let ids: Vec<&str> = all.iter().filter_map(id).collect();
    for (n, (wanted, tag)) in TAGGED.iter().enumerate() {
        assert!(
            ids.contains(wanted) || keys.iter().any(|k| k == wanted),
            "{wanted} is no scenario"
        );
        assert!(
            !TAGGED[..n].iter().any(|(k, _)| k == wanted),
            "{wanted} is tagged twice"
        );
        let kinds = ["divergence_", "unsupported_", "unbound"];
        assert!(
            kinds.iter().any(|kind| tag.name.starts_with(kind)),
            "{wanted}: {}",
            tag.name
        );
    }
}
