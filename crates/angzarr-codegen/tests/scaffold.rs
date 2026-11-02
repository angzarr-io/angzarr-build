//! The scaffold, split so the user never edits a generated file and a run
//! never rewrites the user's: per component its interface (a trait over the
//! domain types) in the generated domain layer, and its implementation, the
//! user's, written once (`angzarr scaffold rust`) and never overwritten,
//! `--missing` adding the required methods it lacks; and the adapters, the
//! CLI's, regenerated every build, carrying angzarr-embedded's markup and
//! delegating to the interface. Pinned by golden files
//! (`tests/golden/rust/buyin`, `tests/golden/buyin.adapters.rs`); that the
//! adapters compile against angzarr-embedded is angzarr-build's
//! `tests/local.rs`.

mod common;

use angzarr_codegen::{
    CodegenError, Existing, MarkupError, Notes, Owner, ScaffoldFile, Translate, UseError, adapters,
    complete, markup_domain, scaffold,
};
use common::*;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

/// The buy-in project's scaffold, with `translate`.
fn buyin(translate: &[Translate]) -> Vec<ScaffoldFile> {
    scaffold(&descriptors(BUYIN), BUYIN, translate).unwrap()
}

/// The domain layer of `files`' one package, its interfaces in it.
fn layer(files: &[&str], package: &str) -> String {
    markup_domain(&descriptors(files), files, &[(package, "crate::wire")])
        .unwrap()
        .into_iter()
        .find(|g| g.package == package)
        .map(|g| g.source)
        .unwrap_or_else(|| panic!("no layer of {package}"))
}

/// The buy-in's domain layer.
fn buyin_layer() -> String {
    layer(BUYIN, "buyin.v1")
}

fn file_at<'a>(files: &'a [ScaffoldFile], path: &str) -> &'a ScaffoldFile {
    files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path}: {files:#?}"))
}

/// The refusal switch set to translate for BuyInWallet's Hold, handing the
/// buy-in its own HoldLost.
fn translate_hold() -> Translate {
    Translate {
        policy: "buyin.v1.BuyInWalletPolicy".to_string(),
        command: "buyin.v1.Hold".to_string(),
        fact: "buyin.v1.HoldLost".to_string(),
    }
}

/// `actual`, against the golden file at `path` under `tests/golden`;
/// `ANGZARR_BLESS=1` rewrites the golden file instead.
fn golden(path: &str, actual: &str) {
    let path = format!("{GOLDEN}/{path}");
    if std::env::var_os("ANGZARR_BLESS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("no {path}"));
    assert_eq!(actual, expected, "{path}");
}

#[test]
fn test_scaffold_writes_an_implementation_per_component_and_reports_the_old_interface() {
    let mut placed: Vec<(String, String, Owner)> = buyin(&[])
        .into_iter()
        .map(|f| (f.component, f.path, f.owner))
        .collect();
    placed.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let at = |c: &str, p: &str, o: Owner| (c.to_string(), p.to_string(), o);
    assert_eq!(
        placed,
        [
            at("buyin.v1.BuyInState", "buy_in.rs", Owner::Obsolete),
            at("buyin.v1.BuyInState", "buy_in_impl.rs", Owner::User),
            at(
                "buyin.v1.BuyInWalletPolicy",
                "buy_in_wallet.rs",
                Owner::Obsolete
            ),
            at(
                "buyin.v1.BuyInWalletPolicy",
                "buy_in_wallet_impl.rs",
                Owner::User
            ),
            at(
                "buyin.v1.WalletBuyInPolicy",
                "wallet_buy_in.rs",
                Owner::Obsolete
            ),
            at(
                "buyin.v1.WalletBuyInPolicy",
                "wallet_buy_in_impl.rs",
                Owner::User
            ),
            at("buyin.v1.WalletState", "wallet.rs", Owner::Obsolete),
            at("buyin.v1.WalletState", "wallet_impl.rs", Owner::User),
        ]
    );
}

#[test]
fn test_scaffold_matches_the_golden_files() {
    for f in buyin(&[]).iter().filter(|f| f.owner == Owner::User) {
        golden(&format!("rust/buyin/{}", f.path), &f.source);
    }
}

/// Each component's interface is the domain layer's: the trait the
/// adapter delegates to and the implementation implements, generated with
/// the protos.
#[test]
fn test_the_interfaces_are_the_domain_layers() {
    let layer = buyin_layer();
    for name in ["BuyIn", "Wallet", "BuyInWallet", "WalletBuyIn"] {
        assert!(
            layer.contains(&format!("pub trait {name} {{")),
            "{name}:\n{layer}"
        );
    }
    golden("buyin.domain.rs", &layer);
    // Each carries the allowance its handlers' `Failed` needs, so no crate
    // implementing it does at the trait's include site.
    assert_eq!(
        layer
            .matches("#[allow(clippy::result_large_err)]\npub trait ")
            .count(),
        4,
        "{layer}"
    );
}

/// A user file imports nothing from the runtime directly: its context and
/// error types come through `crate::domain`, and no code line names the
/// runtime's crates, nor its markup.
#[test]
fn test_the_users_files_import_nothing_from_the_runtime_directly() {
    for f in buyin(&[translate_hold()]) {
        let code: Vec<&str> = f
            .source
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect();
        for line in &code {
            for runtime in [
                "angzarr_embedded",
                "angzarr_",
                "#[on_",
                "#[handles",
                "#[policy",
            ] {
                assert!(
                    !line.contains(runtime),
                    "{} names {runtime}: {line}",
                    f.path
                );
            }
            if let Some(path) = line.trim_start().strip_prefix("use ") {
                assert!(path.starts_with("crate::"), "{} imports {path}", f.path);
            }
        }
    }
}

#[test]
fn test_every_implementation_body_is_todo_and_every_parameter_unused() {
    for f in buyin(&[translate_hold()])
        .iter()
        .filter(|f| f.path.ends_with("_impl.rs"))
    {
        let file = syn::parse_file(&f.source).unwrap();
        let imp = file
            .items
            .iter()
            .find_map(|i| match i {
                syn::Item::Impl(imp) if imp.trait_.is_some() => Some(imp),
                _ => None,
            })
            .expect("the trait impl");
        for f in methods(imp) {
            assert_eq!(text(&f.block), "{todo!()}");
            for input in &f.sig.inputs {
                if let syn::FnArg::Typed(t) = input {
                    assert!(text(&t.pat).starts_with('_'), "{}", text(&f.sig));
                }
            }
        }
    }
}

#[test]
fn test_an_interface_takes_a_fact_as_fact_and_a_command_as_cmd() {
    let source = buyin_layer();
    assert!(
        source.contains("fn hold_lost(&self, fact: &HoldLost, ctx: &Context) -> Vec<BuyInEvent>;"),
        "a fact's decision is infallible:\n{source}"
    );
    assert!(
        source.contains(
            "    fn record_held(\n        &self,\n        cmd: &RecordHeld,\n        ctx: &Context,\n    ) -> Result<Vec<BuyInEvent>, Failed<BuyInRefused>>;"
        ),
        "{source}"
    );
}

/// A policy whose source takes facts has a required `<command>_refused`
/// per command of its target, answering one of the source's facts or a
/// Bug, never declining (decision 14.76; C-0612).
#[test]
fn test_a_policys_interface_requires_each_refusal_callback() {
    let source = buyin_layer();
    assert!(
        source.contains(
            "    fn hold_refused(\n        &self,\n        command: &Hold,\n        refusal: Option<InsufficientFunds>,\n        ctx: &CommandContext,\n    ) -> Result<BuyInFact, Bug>;"
        ),
        "{source}"
    );
    assert!(!source.contains("Ok(None)"), "{source}");
    let wallet_buy_in = &source[source.find("pub trait WalletBuyIn").unwrap()..];
    for callback in ["fn start_buy_in_refused(", "fn record_held_refused("] {
        assert!(
            wallet_buy_in.contains(callback),
            "{callback}:\n{wallet_buy_in}"
        );
    }
    assert!(
        wallet_buy_in.contains(") -> Result<WalletFact, Bug>;"),
        "{wallet_buy_in}"
    );
}

/// The implementation is created with every refusal callback, each a
/// `todo!()`: none is optional (decision 14.76).
#[test]
fn test_scaffold_writes_a_refused_method_for_each_command_of_the_target() {
    let files = buyin(&[]);
    let implementation = file_at(&files, "buy_in_wallet_impl.rs");
    let source = &implementation.source;
    assert!(
        source.contains(
            "    fn hold_refused(\n        &self,\n        _command: &Hold,\n        _refusal: Option<InsufficientFunds>,\n        _ctx: &CommandContext,\n    ) -> Result<BuyInFact, Bug> {\n        todo!()\n    }"
        ),
        "{source}"
    );
    assert!(source.contains("fn deposit_refused("), "{source}");
    assert!(implementation.optional.is_empty());
    assert!(implementation.selected.is_empty());
}

#[test]
fn test_scaffold_refuses_translate_for_an_unknown_policy() {
    let selection = Translate {
        policy: "buyin.v1.Nowhere".to_string(),
        ..translate_hold()
    };
    assert_eq!(
        scaffold(&descriptors(BUYIN), BUYIN, &[selection]),
        Err(CodegenError::Use(UseError::TranslateUnknownPolicy {
            policy: "buyin.v1.Nowhere".to_string(),
        }))
    );
}

#[test]
fn test_scaffold_refuses_translate_for_a_command_its_target_does_not_take() {
    let selection = Translate {
        command: "buyin.v1.StartBuyIn".to_string(),
        ..translate_hold()
    };
    assert_eq!(
        scaffold(&descriptors(BUYIN), BUYIN, &[selection]),
        Err(CodegenError::Use(UseError::TranslateNotATargetCommand {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            command: "buyin.v1.StartBuyIn".to_string(),
        }))
    );
}

#[test]
fn test_scaffold_refuses_translate_to_a_fact_its_source_does_not_take() {
    let selection = Translate {
        fact: "buyin.v1.Deposit".to_string(),
        ..translate_hold()
    };
    assert_eq!(
        scaffold(&descriptors(BUYIN), BUYIN, &[selection]),
        Err(CodegenError::Use(UseError::TranslateNotASourceFact {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            fact: "buyin.v1.Deposit".to_string(),
        }))
    );
}

#[test]
fn test_scaffold_refuses_translate_to_a_command_of_its_source() {
    let selection = Translate {
        fact: "buyin.v1.RecordHeld".to_string(),
        ..translate_hold()
    };
    assert_eq!(
        scaffold(&descriptors(BUYIN), BUYIN, &[selection]),
        Err(CodegenError::Use(UseError::TranslateNotAFact {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            fact: "buyin.v1.RecordHeld".to_string(),
        }))
    );
}

#[test]
fn test_scaffold_refuses_refuses_on_a_fact() {
    const FILE: &str = "bad/v1/refusing_fact.proto";
    assert_eq!(
        scaffold(&descriptors(&[FILE]), &[FILE], &[]),
        Err(CodegenError::Markup(MarkupError::RefusesOnFact {
            message: "bad.v1.Happened".to_string(),
        }))
    );
}

// --------------------------------------------------------------- blackjack

#[test]
fn test_an_interface_of_one_file_finds_its_policies_owners_and_events_elsewhere() {
    // policies.proto alone is scaffolded: the table and the player owning
    // the policies' domains, and the events they react to, are declared in
    // other files, as the domain layer's interfaces are.
    const POLICIES: &str = "io/angzarr/examples/v1/policies.proto";
    let files = scaffold(&descriptors(BLACKJACK), &[POLICIES], &[]).unwrap();
    file_at(&files, "player_table_policy_impl.rs");
    let source = layer(BLACKJACK, "io.angzarr.examples.v1");
    let player_table = &source[source.find("pub trait PlayerTablePolicy").unwrap()..];
    assert!(player_table.contains("fn move_decided("), "{player_table}");
    // What it sends the table: its outputs, commands and facts alike
    // (decision 14.69).
    assert!(
        player_table.contains("TableAggregateOutput"),
        "{player_table}"
    );
}

#[test]
fn test_a_decision_returns_a_list_of_its_events() {
    const TABLE: &str = "io/angzarr/examples/v1/table.proto";
    let source = layer(BLACKJACK, "io.angzarr.examples.v1");
    let table = &source[source.find("pub trait TableAggregate").unwrap()..];
    // Decision 14.75: every handling returns a list of the table's events
    // (C-0797), and the table refuses with TableRefused.
    for expected in [
        "        cmd: &PlaceBet,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        "        cmd: &Hit,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        "        cmd: &CreateTable,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        "        cmd: &DealRound,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        "        cmd: &ConfirmSeat,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<SeatNotConfirmable>>;",
        // ReleaseSeat is a fact (ruling 9, decision 14.69): recorded, never
        // refused.
        "fn release_seat(&self, fact: &ReleaseSeat, ctx: &Context) -> Vec<TableAggregateEvent>;",
    ] {
        assert!(table.contains(expected), "{expected}:\n{table}");
    }
    let files = scaffold(&descriptors(BLACKJACK), &[TABLE], &[]).unwrap();
    let state = &file_at(&files, "table_aggregate_impl.rs").source;
    assert!(
        state.contains("pub state: crate::wire::TableState,"),
        "{state}"
    );
}

// ------------------------------------------------------------- adapters

#[test]
fn test_adapters_match_the_golden_file() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    golden("buyin.adapters.rs", &source);
}

#[test]
fn test_adapters_name_each_policy_by_its_domains() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    for name in [
        "#[policy(name = \"policy-buyin-wallet\", source = BuyIn, target = Wallet)]",
        "#[policy(name = \"policy-wallet-buyin\", source = Wallet, target = BuyIn)]",
    ] {
        assert!(source.contains(name), "{name}:\n{source}");
    }
}

/// A policy whose source takes facts has a refusal callback per command of
/// its target, each delegating to its interface's required
/// `<command>_refused`, whose fact the markup always gets: the generated
/// policy never declines (decision 14.76).
#[test]
fn test_adapters_wire_a_translating_refusal_callback_per_command() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    let file = parse_adapters(&source);
    let imp = inherent_impl(&file, "BuyInWallet");
    assert_eq!(
        markers(imp),
        s(&[
            ("on_event", "BuyInStarted"),
            ("on_refused", "Deposit"),
            ("on_refused", "Hold"),
        ])
    );
    // The callback takes the refused command's context and answers the
    // source's fact, never declining (the markup's non-declining form):
    // the interface's method reads the context and answers it, or a Bug.
    let callback = methods(imp)[2];
    assert_eq!(
        callback
            .sig
            .inputs
            .iter()
            .skip(1)
            .map(|input| match input {
                syn::FnArg::Typed(t) => text(&t.ty),
                syn::FnArg::Receiver(_) => String::new(),
            })
            .collect::<Vec<_>>(),
        ["&Refusal<Hold>", "&CommandContext"]
    );
    assert_eq!(
        text(&callback.sig.output),
        "->::core::result::Result<BuyInFact,Bug>"
    );
    assert_eq!(
        text(&callback.block),
        "{crate::domain::BuyInWallet::hold_refused(&self.0,refusal.command(),refusal.refusal::<InsufficientFunds>(),ctx,)}"
    );
    assert_eq!(
        text(&methods(imp)[1].block),
        "{crate::domain::BuyInWallet::deposit_refused(&self.0,refusal.command(),refusal.refusal::<DepositRefused>(),ctx,)}"
    );
    // The wallet's policy to the buy-in: a callback per buy-in command.
    let other = inherent_impl(&file, "WalletBuyIn");
    assert_eq!(
        markers(other),
        s(&[
            ("on_event", "Held"),
            ("on_refused", "StartBuyIn"),
            ("on_refused", "RecordHeld"),
        ])
    );
}

/// The kinds fixture's policy translating Audit, which refuses two types,
/// into the ledger's Arrived.
fn translate_audit() -> Translate {
    Translate {
        policy: "kinds.v1.LedgerAuditPolicy".to_string(),
        command: "kinds.v1.Audit".to_string(),
        fact: "kinds.v1.Arrived".to_string(),
    }
}

const KINDS: &[&str] = &["kinds/v1/kinds.proto"];

/// A command refusing several types is refused with one of them: the
/// `<command>_refused` method takes the domain layer's choice of them, as
/// the command's handler refuses with it, not one type or none.
#[test]
fn test_a_refused_method_takes_the_choice_of_a_command_refusing_several_types() {
    let source = layer(KINDS, "kinds.v1");
    assert!(
        source.contains(
            "    fn audit_refused(\n        &self,\n        command: &Audit,\n        refusal: Option<AuditRefuses>,\n        ctx: &CommandContext,\n    ) -> Result<LedgerStateFact, Bug>;"
        ),
        "{source}"
    );
    let files = scaffold(&descriptors(KINDS), KINDS, &[translate_audit()]).unwrap();
    let source = &file_at(&files, "ledger_audit_policy_impl.rs").source;
    assert!(
        source.contains("_refusal: Option<AuditRefuses>,"),
        "{source}"
    );
}

/// The adapter hands that method the refusal as the choice: each of the
/// command's refusal types tried in turn, the one it is chosen.
#[test]
fn test_adapters_hand_a_refused_method_the_choice_of_several_refusal_types() {
    let source = adapters(&descriptors(KINDS), KINDS, &[]).unwrap();
    let file = parse_adapters(&source);
    let imp = inherent_impl(&file, "LedgerAuditPolicy");
    let audit = methods(imp)
        .into_iter()
        .find(|m| m.sig.ident == "audit_refused")
        .expect("the Audit callback");
    assert_eq!(
        text(&audit.block),
        "{crate::domain::LedgerAuditPolicy::audit_refused(&self.0,refusal.command(),\
         refusal.refusal::<PostRefused>().map(AuditRefuses::PostRefused)\
         .or_else(||{refusal.refusal::<Unreasoned>().map(AuditRefuses::Unreasoned)}),ctx,)}"
    );
}

/// A fact handler's adapter hands the interface its context too, with no
/// Result: a fact is recorded, never refused.
#[test]
fn test_adapters_forward_a_fact_handlers_context() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    let file = parse_adapters(&source);
    let buy_in = inherent_impl(&file, "BuyIn");
    let fact = methods(buy_in)
        .into_iter()
        .find(|m| m.sig.ident == "hold_lost")
        .expect("the HoldLost handler");
    assert!(
        text(&fact.block).contains("crate::domain::BuyIn::hold_lost(&self.0,&fact,ctx)"),
        "{}",
        text(&fact.block)
    );
}

/// A command handler's adapter hands the interface its context; a policy
/// reaction's hands it its context and answers its Result.
#[test]
fn test_adapters_forward_each_handlers_context() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    let file = parse_adapters(&source);
    let wallet = inherent_impl(&file, "Wallet");
    assert!(
        text(&methods(wallet)[0].block).starts_with(
            "{crate::domain::Wallet::deposit(&self.0,&cmd,ctx).map(|events:Vec<WalletEvent>|"
        ),
        "{}",
        text(&methods(wallet)[0].block)
    );
    let policy = inherent_impl(&file, "BuyInWallet");
    let reaction = methods(policy)[0];
    assert_eq!(
        text(&reaction.sig.output),
        "->::core::result::Result<Commands<Wallet>,Bug>"
    );
    assert!(
        text(&reaction.block).starts_with(
            "{letorders=crate::domain::BuyInWallet::buy_in_started(&self.0,&ctx.source_root().value,event,ctx,)?;"
        ),
        "{}",
        text(&reaction.block)
    );
}

/// A policy reaction takes the source root, the event and its context, and
/// answers its outputs (its target's commands and facts) or its error, a
/// bug.
#[test]
fn test_an_interface_gives_a_policy_reaction_its_context_and_a_result() {
    let source = buyin_layer();
    assert!(
        source.contains(
            "    fn buy_in_started(\n        &self,\n        source: &[u8],\n        event: &BuyInStarted,\n        ctx: &PolicyContext,\n    ) -> Result<Vec<(Vec<u8>, WalletOutput)>, Bug>;"
        ),
        "{source}"
    );
}

/// A domain's handlers fail with the generated layer's `Failed`, larger
/// than clippy's `result_large_err` allows; the generated module allows it,
/// so no application's include site need (item S).
#[test]
fn test_adapters_allow_a_domains_large_failure() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let module = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Mod(m) if m.ident == "adapters" => Some(m),
            _ => None,
        })
        .expect("the adapters module");
    assert!(
        module
            .attrs
            .iter()
            .any(|a| text(a) == "#[allow(clippy::result_large_err)]"),
        "{source}"
    );
}

/// A policy a child process serves has no adapter here, named by its name
/// in the application; the domains it joins keep theirs.
#[test]
fn test_adapters_leave_a_child_policy_to_its_process() {
    let all = adapters(&descriptors(BUYIN), BUYIN, &[]).unwrap();
    assert!(all.contains("policy-buyin-wallet"), "{all}");
    let source = adapters(&descriptors(BUYIN), BUYIN, &["policy-buyin-wallet"]).unwrap();
    assert!(!source.contains("policy-buyin-wallet"), "{source}");
    assert!(source.contains("policy-wallet-buyin"), "{source}");
}

/// A domain served by a child process is no aggregate here: a policy
/// names it by its command set (`CommandTarget`, with `Decides` of each
/// command) and its event set (`EventSource`, with `Ingests` of each
/// fact), and no stand-in markup is generated for it.
#[test]
fn test_adapters_name_a_remote_domain_by_its_commands_and_events() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &["buyin"]).unwrap();
    let file = parse_adapters(&source);
    let impls: Vec<&syn::ItemImpl> = file
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Impl(imp) if text(&imp.self_ty) == "BuyIn" => Some(imp),
            _ => None,
        })
        .collect();
    assert!(
        impls.iter().all(|imp| imp.trait_.is_some()),
        "no markup impl of the buy-in: {source}"
    );
    let traits: Vec<String> = impls
        .iter()
        .filter_map(|imp| imp.trait_.as_ref().map(|t| text(&t.0)))
        .collect();
    assert_eq!(
        traits,
        [
            "CommandTarget",
            "Decides<StartBuyIn>",
            "Decides<RecordHeld>",
            "EventSource",
            "Ingests<HoldLost>",
        ]
    );
    let body = |name: &str| {
        impls
            .iter()
            .find(|imp| imp.trait_.as_ref().is_some_and(|t| text(&t.0) == name))
            .map(|imp| imp.items.iter().map(text).collect::<String>())
            .unwrap()
    };
    let target = body("CommandTarget");
    assert!(
        target.contains("constDOMAIN:&'staticstr=\"buyin\";"),
        "{target}"
    );
    assert!(
        target.contains(
            "vec![\"buyin.v1.StartBuyIn\".to_string(),\"buyin.v1.RecordHeld\".to_string()]"
        ),
        "{target}"
    );
    let events = body("EventSource");
    assert!(
        events.contains("constDOMAIN:&'staticstr=\"buyin\";"),
        "{events}"
    );
    assert!(
        events.contains(
            "vec![\"buyin.v1.BuyInStarted\".to_string(),\"buyin.v1.BuyInHeld\".to_string(),\"buyin.v1.BuyInFailed\".to_string()]"
        ),
        "{events}"
    );
    assert!(!source.contains("crate::buy_in_impl::"), "{source}");
    assert!(!source.contains("unreachable!"), "{source}");
}

/// The markup the adapters import, by name.
fn markup_imports(source: &str) -> Vec<String> {
    let file = parse_adapters(source);
    let tree = file
        .items
        .iter()
        .find_map(|i| match i {
            syn::Item::Use(u) if text(u).contains("markup::") => Some(text(&u.tree)),
            _ => None,
        })
        .expect("a markup import");
    let (_, names) = tree.rsplit_once('{').unwrap();
    names
        .trim_end_matches('}')
        .split(',')
        .map(str::to_string)
        .collect()
}

/// A remote domain's import takes `Decides` only when a command of its own
/// reaches it, `Ingests` only when a fact does: the kinds audit, commands
/// alone, imports `Decides` and no `Ingests`; the buy-in, a fact as well,
/// both.
#[test]
fn test_adapters_import_a_remote_domains_markup_by_its_inputs() {
    let audit = adapters(&descriptors(KINDS), KINDS, &["audit"]).unwrap();
    let imports = markup_imports(&audit);
    assert!(imports.iter().any(|m| m == "Decides"), "{imports:?}");
    assert!(!imports.iter().any(|m| m == "Ingests"), "{imports:?}");
    let buyin = adapters(&descriptors(BUYIN), BUYIN, &["buyin"]).unwrap();
    let imports = markup_imports(&buyin);
    assert!(imports.iter().any(|m| m == "Decides"), "{imports:?}");
    assert!(imports.iter().any(|m| m == "Ingests"), "{imports:?}");
}

// ------------------------------------------------------------- --missing

/// The wallet's implementation as a user has it: Hold and Held absent (the
/// proto gained them later), a method and a doc comment of the user's own,
/// and the decision of a command the proto no longer declares.
const USER_WALLET_IMPL: &str = r#"//! The wallet, mine.

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct WalletImpl {
    pub state: crate::wire::WalletState,
}

impl Wallet for WalletImpl {
    /// The user's own doc, which `--missing` must keep byte for byte.
    fn deposit(&self, cmd: &Deposit, _ctx: &Context) -> Result<Vec<WalletEvent>, Failed<DepositRefused>> {
        Ok(vec![Deposited { cents: cmd.cents }.into()]) // USER-WRITTEN
    }

    fn record_buy_in_refused(&self, _fact: &RecordBuyInRefused, _ctx: &Context) -> Vec<WalletEvent> {
        Vec::new()
    }

    fn apply_buy_in_refusal_recorded(&mut self, _event: &BuyInRefusalRecorded) {}

    fn withdraw(&self, cmd: &Withdraw, _ctx: &Context) -> Result<Deposited, Failed<Rejection>> {
        todo!()
    }

    fn apply_deposited(&mut self, event: &Deposited) {
        self.state.balance += event.cents;
    }
}
"#;

#[test]
fn test_complete_adds_only_the_absent_required_methods_to_the_implementation() {
    let files = buyin(&[]);
    let completion = complete(
        file_at(&files, "wallet_impl.rs"),
        USER_WALLET_IMPL,
        Existing::Complete,
    )
    .unwrap();
    let expected = USER_WALLET_IMPL.replace(
        "        self.state.balance += event.cents;\n    }\n}",
        "        self.state.balance += event.cents;\n    }\n\n    \
         fn hold(\n        &self,\n        _cmd: &Hold,\n        _ctx: &Context,\n    ) -> Result<Vec<WalletEvent>, Failed<InsufficientFunds>> {\n        todo!()\n    }\n\n    \
         fn apply_held(&mut self, _event: &Held) {\n        todo!()\n    }\n}",
    );
    assert_eq!(completion.completed, Some(expected));
    assert_eq!(
        completion.notes,
        Notes {
            obsolete: vec!["withdraw".to_string()],
            ..Notes::default()
        }
    );
}

/// Kept, a file is never written: what it lacks is reported, and what it
/// has for a message the protos no longer declare.
#[test]
fn test_complete_of_a_kept_file_reports_what_it_lacks_and_what_is_obsolete() {
    let files = buyin(&[]);
    let completion = complete(
        file_at(&files, "wallet_impl.rs"),
        USER_WALLET_IMPL,
        Existing::Keep,
    )
    .unwrap();
    assert_eq!(completion.completed, None);
    assert_eq!(
        completion.notes,
        Notes {
            lacks: vec!["hold".to_string(), "apply_held".to_string()],
            obsolete: vec!["withdraw".to_string()],
            ..Notes::default()
        }
    );
}

/// A policy's refusal callback is required (decision 14.76): `--missing`
/// adds one an existing implementation lacks, as any required method, and
/// a kept file reports it.
#[test]
fn test_complete_adds_a_missing_refused_method() {
    let plain = buyin(&[]);
    let path = "buy_in_wallet_impl.rs";
    let fresh = file_at(&plain, path);
    let start = fresh.source.find("    fn hold_refused(").unwrap();
    let end = start + fresh.source[start..].find("    }\n").unwrap() + "    }\n".len();
    let mine = format!("{}{}", &fresh.source[..start], &fresh.source[end..]);
    let kept = complete(fresh, &mine, Existing::Keep).unwrap();
    assert_eq!(kept.completed, None);
    assert_eq!(kept.notes.lacks, ["hold_refused"]);
    assert!(kept.notes.optional.is_empty());
    let completed = complete(fresh, &mine, Existing::Complete)
        .unwrap()
        .completed
        .expect("the required callback is added");
    assert!(completed.contains("fn hold_refused("), "{completed}");
}

#[test]
fn test_complete_refuses_a_command_taken_as_a_fact() {
    let files = buyin(&[]);
    let fresh = file_at(&files, "buy_in_impl.rs");
    let as_fact = fresh
        .source
        .replace("_cmd: &RecordHeld", "_fact: &RecordHeld");
    assert_ne!(as_fact, fresh.source);
    assert_eq!(
        complete(fresh, &as_fact, Existing::Keep),
        Err(CodegenError::Use(UseError::InputMismatch {
            path: "buy_in_impl.rs".to_string(),
            message: "buyin.v1.RecordHeld".to_string(),
            expected: "cmd".to_string(),
            found: "fact".to_string(),
        }))
    );
}

#[test]
fn test_complete_refuses_a_file_without_the_components_impl() {
    let files = buyin(&[]);
    assert_eq!(
        complete(
            file_at(&files, "wallet_impl.rs"),
            "//! Not the wallet any more.\n",
            Existing::Keep
        ),
        Err(CodegenError::Use(UseError::ScaffoldImplNotFound {
            path: "wallet_impl.rs".to_string(),
            component: "buyin.v1.WalletState".to_string(),
        }))
    );
}

#[test]
fn test_complete_inserts_before_a_closing_brace_that_ends_a_line() {
    let files = buyin(&[]);
    let user = USER_WALLET_IMPL.replace(
        "        self.state.balance += event.cents;\n    }\n}\n",
        "        self.state.balance += event.cents;\n    } }\n",
    );
    assert_ne!(user, USER_WALLET_IMPL);
    let completed = complete(file_at(&files, "wallet_impl.rs"), &user, Existing::Complete)
        .unwrap()
        .completed
        .expect("hold and apply_held are absent");
    assert!(
        completed.ends_with(
            "    } \n\n    fn hold(\n        &self,\n        _cmd: &Hold,\n        _ctx: &Context,\n    ) -> Result<Vec<WalletEvent>, Failed<InsufficientFunds>> {\n        todo!()\n    }\n\n    \
             fn apply_held(&mut self, _event: &Held) {\n        todo!()\n    }\n}\n"
        ),
        "{completed}"
    );
}

#[test]
fn test_complete_takes_a_method_naming_its_event_otherwise_as_present() {
    let files = buyin(&[]);
    let fresh = file_at(&files, "wallet_impl.rs");
    let renamed = fresh
        .source
        .replace("_event: &Deposited", "deposited: &Deposited");
    assert_ne!(renamed, fresh.source);
    assert_eq!(
        complete(fresh, &renamed, Existing::Complete)
            .unwrap()
            .completed,
        None
    );
}

/// A manifest is never completed: its pins that are not this CLI's are
/// reported (item S Q8).
#[test]
fn test_complete_of_a_manifest_reports_its_pins_that_drift() {
    let fresh = ScaffoldFile {
        component: String::new(),
        path: "Cargo.toml".to_string(),
        source: "[dependencies]\nangzarr-embedded = { git = \"g\", rev = \"new\" }\nother = { git = \"o\", rev = \"x\" }\n".to_string(),
        owner: Owner::User,
        optional: Vec::new(),
        selected: Vec::new(),
    };
    let mine = "[dependencies]\nangzarr-embedded = { git = \"g\", rev = \"old\" }\nother = { path = \"../o\" }\n";
    let completion = complete(&fresh, mine, Existing::Complete).unwrap();
    assert_eq!(completion.completed, None);
    assert_eq!(
        completion.notes.drift,
        [angzarr_codegen::PinDrift {
            dependency: "angzarr-embedded".to_string(),
            pinned: "old".to_string(),
            cli: "new".to_string(),
        }]
    );
    assert_eq!(
        complete(&fresh, &fresh.source, Existing::Keep)
            .unwrap()
            .notes,
        Notes::default()
    );
}
