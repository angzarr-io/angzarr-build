//! The generated layer names only angzarr-embedded's public, documented
//! API: every path into the runtime's crates in the generated domain
//! layers, adapters, registrations and descriptors of the fixtures and the
//! spec's blackjack protos is one of [`ALLOWED`], and none is a `__`
//! item. A hand-written component can name everything generated code
//! does, so generated and hand-written code stay interchangeable.

mod common;

use std::collections::BTreeSet;

use angzarr_codegen::{adapters, descriptors_rs, markup_domain, register};
use common::*;
use proc_macro2::{TokenStream, TokenTree};

/// Every runtime path the generator may emit, each public and documented
/// in angzarr-embedded (`next`, `next::markup`), its macros and its
/// diagnostics.
const ALLOWED: &[&str] = &[
    "angzarr_embedded::next::Registry",
    "angzarr_embedded::next::Timestamp",
    "angzarr_embedded::next::WireError",
    "angzarr_embedded::next::WireError::Missing",
    "angzarr_embedded::next::WireError::UnknownEnum",
    "angzarr_embedded::next::v1::Uuid",
    "angzarr_embedded::next::markup::BusinessStatus",
    "angzarr_embedded::next::markup::CommandContext",
    "angzarr_embedded::next::markup::Commands",
    "angzarr_embedded::next::markup::Context",
    "angzarr_embedded::next::markup::DomainMessage",
    "angzarr_embedded::next::markup::Emit",
    "angzarr_embedded::next::markup::Failed",
    "angzarr_embedded::next::markup::PolicyContext",
    "angzarr_embedded::next::markup::ProjectionContext",
    "angzarr_embedded::next::markup::Record",
    "angzarr_embedded::next::markup::Refusal",
    "angzarr_embedded::next::markup::Refuses",
    "angzarr_embedded::next::markup::Report",
    "angzarr_embedded::next::markup::RetryableStatus",
    "angzarr_embedded_diagnostic::Arg",
    "angzarr_embedded_diagnostic::Arg::from",
    "angzarr_embedded_diagnostic::Diagnostic",
    "angzarr_embedded_diagnostic::Diagnostic::args",
    "angzarr_embedded_diagnostic::Diagnostic::code",
    "angzarr_embedded_macros::aggregate",
    "angzarr_embedded_macros::applies",
    "angzarr_embedded_macros::handles_cmd",
    "angzarr_embedded_macros::handles_fact",
    "angzarr_embedded_macros::on_event",
    "angzarr_embedded_macros::on_refused",
    "angzarr_embedded_macros::policy",
    "angzarr_embedded_macros::process_manager",
    "angzarr_embedded_macros::projector",
    "angzarr_embedded_macros::projects",
];

/// Every path into a crate named `angzarr_embedded*` in `tokens`, a `use`
/// group's members each its own path.
fn runtime_paths(tokens: TokenStream, out: &mut BTreeSet<String>) {
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut i = 0;
    while i < trees.len() {
        if let TokenTree::Ident(id) = &trees[i]
            && id.to_string().starts_with("angzarr_embedded")
        {
            let mut path = id.to_string();
            i = follow(&trees, i + 1, &mut path, out);
            continue;
        }
        if let TokenTree::Group(group) = &trees[i] {
            runtime_paths(group.stream(), out);
        }
        i += 1;
    }
}

/// `path` extended by the `::segment`s from `i`, a trailing `::{..}`
/// group expanded: each full path into `out`. The index after it.
fn follow(
    trees: &[TokenTree],
    mut i: usize,
    path: &mut String,
    out: &mut BTreeSet<String>,
) -> usize {
    loop {
        let colons = matches!(
            (trees.get(i), trees.get(i + 1)),
            (Some(TokenTree::Punct(a)), Some(TokenTree::Punct(b))) if a.as_char() == ':' && b.as_char() == ':'
        );
        if !colons {
            out.insert(path.clone());
            return i;
        }
        match trees.get(i + 2) {
            Some(TokenTree::Ident(segment)) => {
                path.push_str(&format!("::{segment}"));
                i += 3;
            }
            Some(TokenTree::Group(group)) => {
                let members: Vec<TokenTree> = group.stream().into_iter().collect();
                let mut j = 0;
                while j < members.len() {
                    if let TokenTree::Ident(id) = &members[j] {
                        let mut member = format!("{path}::{id}");
                        j = follow(&members, j + 1, &mut member, out);
                    } else {
                        j += 1;
                    }
                }
                return i + 3;
            }
            _ => {
                out.insert(path.clone());
                return i + 2;
            }
        }
    }
}

fn generated(files: &[&str], wire: &[(&str, &str)], remote: &[&str]) -> Vec<String> {
    let set = descriptors(files);
    let mut sources: Vec<String> = markup_domain(&set, files, wire)
        .unwrap()
        .into_iter()
        .map(|g| g.source)
        .collect();
    sources.push(adapters(&set, files, remote, &|_| None).unwrap());
    sources.push(register(&set, files, remote).unwrap());
    sources.push(descriptors_rs(&set, files, remote, "1").unwrap());
    sources
}

#[test]
fn test_generated_code_names_only_the_runtimes_public_api() {
    let mut found = BTreeSet::new();
    for source in [
        generated(BUYIN, &[("buyin.v1", "crate::wire")], &[]),
        generated(
            &["kinds/v1/kinds.proto"],
            &[("kinds.v1", "crate::wire")],
            &[],
        ),
        generated(
            BLACKJACK,
            &[("io.angzarr.examples.v1", "crate::wire")],
            &["player"],
        ),
    ]
    .concat()
    {
        let tokens: TokenStream = source.parse().expect("generated source tokenizes");
        runtime_paths(tokens, &mut found);
    }
    let private: Vec<&String> = found
        .iter()
        .filter(|p| p.split("::").any(|s| s.starts_with("__")))
        .collect();
    assert!(private.is_empty(), "private runtime items: {private:?}");
    let unknown: Vec<&String> = found
        .iter()
        .filter(|p| !ALLOWED.contains(&p.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "runtime paths outside the allowlist: {unknown:?}"
    );
    // The walk sees the paths it is checking.
    for expected in [
        "angzarr_embedded::next::markup::Emit",
        "angzarr_embedded_macros::projector",
        "angzarr_embedded::next::Registry",
    ] {
        assert!(found.contains(expected), "{expected}: {found:?}");
    }
}

#[test]
fn test_the_walk_finds_each_path_and_each_use_groups_member() {
    let mut found = BTreeSet::new();
    let tokens: TokenStream = "use ::angzarr_embedded::next::markup::{Emit, Record}; \
         fn f() { ::angzarr_embedded::next::markup::__pack(x); }"
        .parse()
        .unwrap();
    runtime_paths(tokens, &mut found);
    assert_eq!(
        found.into_iter().collect::<Vec<_>>(),
        [
            "angzarr_embedded::next::markup::Emit",
            "angzarr_embedded::next::markup::Record",
            "angzarr_embedded::next::markup::__pack",
        ]
    );
}
