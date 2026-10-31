//! The scaffold, split so the user's logic never names the runtime: per
//! component an interface (a trait over the domain types) and its
//! implementation, the user's, written once (`angzarr scaffold rust`) and
//! completed, never overwritten (`--missing`); and the adapters, the
//! CLI's, regenerated every build, carrying angzarr-embedded's markup and
//! delegating to the interface. Pinned by golden files
//! (`tests/golden/rust/buyin`, `tests/golden/buyin.adapters.rs`); that the
//! adapters compile against angzarr-embedded is angzarr-build's
//! `tests/local.rs`.

mod common;

use angzarr_codegen::{CodegenError, Owner, ScaffoldFile, Translate, adapters, complete, scaffold};
use common::*;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");

/// The buy-in project's scaffold, with `translate`.
fn buyin(translate: &[Translate]) -> Vec<ScaffoldFile> {
    scaffold(&descriptors(BUYIN), BUYIN, translate).unwrap()
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
fn test_scaffold_writes_an_interface_and_an_implementation_per_component() {
    let mut placed: Vec<(String, String, Owner)> = buyin(&[])
        .into_iter()
        .map(|f| (f.component, f.path, f.owner))
        .collect();
    placed.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let user = |c: &str, p: &str| (c.to_string(), p.to_string(), Owner::User);
    assert_eq!(
        placed,
        [
            user("buyin.v1.BuyInState", "buy_in.rs"),
            user("buyin.v1.BuyInState", "buy_in_impl.rs"),
            user("buyin.v1.BuyInWalletPolicy", "buy_in_wallet.rs"),
            user("buyin.v1.BuyInWalletPolicy", "buy_in_wallet_impl.rs"),
            user("buyin.v1.WalletBuyInPolicy", "wallet_buy_in.rs"),
            user("buyin.v1.WalletBuyInPolicy", "wallet_buy_in_impl.rs"),
            user("buyin.v1.WalletState", "wallet.rs"),
            user("buyin.v1.WalletState", "wallet_impl.rs"),
        ]
    );
}

#[test]
fn test_scaffold_matches_the_golden_files() {
    for f in buyin(&[]) {
        golden(&format!("rust/buyin/{}", f.path), &f.source);
    }
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
fn test_scaffold_takes_a_fact_as_fact_and_a_command_as_cmd() {
    let files = buyin(&[]);
    let source = &file_at(&files, "buy_in.rs").source;
    assert!(
        source.contains("fn hold_lost(&self, fact: &HoldLost, ctx: &Context) -> BuyInFailed;"),
        "a fact's decision is infallible:\n{source}"
    );
    assert!(
        source.contains(
            "fn record_held(&self, cmd: &RecordHeld, ctx: &Context) -> Result<BuyInHeld, Failed<Rejection>>;"
        ),
        "{source}"
    );
}

#[test]
fn test_scaffold_adds_a_refused_method_for_each_command_the_switch_translates() {
    let files = buyin(&[translate_hold()]);
    let source = &file_at(&files, "buy_in_wallet.rs").source;
    assert!(
        source.contains(
            "    fn hold_refused(\n        &self,\n        command: &Hold,\n        refusal: Option<InsufficientFunds>,\n        ctx: &CommandContext,\n    ) -> Result<HoldLost, Bug>;"
        ),
        "{source}"
    );
    let other = &file_at(&files, "wallet_buy_in.rs").source;
    assert!(!other.contains("_refused"), "{other}");
}

#[test]
fn test_scaffold_refuses_translate_for_an_unknown_policy() {
    let selection = Translate {
        policy: "buyin.v1.Nowhere".to_string(),
        ..translate_hold()
    };
    assert_eq!(
        scaffold(&descriptors(BUYIN), BUYIN, &[selection]),
        Err(CodegenError::TranslateUnknownPolicy {
            policy: "buyin.v1.Nowhere".to_string(),
        })
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
        Err(CodegenError::TranslateNotATargetCommand {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            command: "buyin.v1.StartBuyIn".to_string(),
        })
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
        Err(CodegenError::TranslateNotASourceFact {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            fact: "buyin.v1.Deposit".to_string(),
        })
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
        Err(CodegenError::TranslateNotAFact {
            policy: "buyin.v1.BuyInWalletPolicy".to_string(),
            fact: "buyin.v1.RecordHeld".to_string(),
        })
    );
}

#[test]
fn test_scaffold_refuses_refuses_on_a_fact() {
    const FILE: &str = "bad/v1/refusing_fact.proto";
    assert_eq!(
        scaffold(&descriptors(&[FILE]), &[FILE], &[]),
        Err(CodegenError::RefusesOnFact {
            message: "bad.v1.Happened".to_string(),
        })
    );
}

// --------------------------------------------------------------- blackjack

#[test]
fn test_scaffold_of_one_file_finds_its_policies_owners_and_events_elsewhere() {
    // policies.proto alone: the table and the player owning the policies'
    // domains, and the events they react to, are declared in other files.
    const POLICIES: &str = "io/angzarr/examples/v1/policies.proto";
    let files = scaffold(&descriptors(BLACKJACK), &[POLICIES], &[]).unwrap();
    let player_table = &file_at(&files, "player_table_policy.rs").source;
    assert!(player_table.contains("fn move_decided("), "{player_table}");
    assert!(
        player_table.contains("TableAggregateCommand"),
        "{player_table}"
    );
}

#[test]
fn test_a_decision_returns_its_one_event_or_any_of_its_events() {
    const TABLE: &str = "io/angzarr/examples/v1/table.proto";
    let files = scaffold(&descriptors(BLACKJACK), &[TABLE], &[]).unwrap();
    let table = &file_at(&files, "table_aggregate.rs").source;
    // Decision 14.50: the result follows the emits cardinality, and the
    // table refuses with TableRefused.
    for expected in [
        "fn place_bet(&self, cmd: &PlaceBet, ctx: &Context) -> Result<BetPlaced, Failed<TableRefused>>;",
        // Hit emits a variable subset of its four (the card, then a turn or
        // the dealer and the settlement), MANY: any of its events.
        "        cmd: &Hit,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        // CreateTable is ONE of two: the tuple.
        "        cmd: &CreateTable,\n        ctx: &Context,\n    ) -> Result<(TableCreated, ShoeShuffled), Failed<TableRefused>>;",
        "        cmd: &DealRound,\n        ctx: &Context,\n    ) -> Result<Vec<TableAggregateEvent>, Failed<TableRefused>>;",
        // ConfirmSeat is OPTIONAL: a repeat emits nothing.
        "        cmd: &ConfirmSeat,\n        ctx: &Context,\n    ) -> Result<Option<PlayerSeated>, Failed<SeatNotConfirmable>>;",
        "        cmd: &ReleaseSeat,\n        ctx: &Context,\n    ) -> Result<Option<SeatReleased>, Failed<Rejection>>;",
    ] {
        assert!(table.contains(expected), "{expected}:\n{table}");
    }
    let state = &file_at(&files, "table_aggregate_impl.rs").source;
    assert!(
        state.contains("pub state: crate::wire::TableState,"),
        "{state}"
    );
}

// ------------------------------------------------------------- adapters

/// The user's buy-in files as the golden scaffold has them.
fn golden_interface(path: &str) -> Option<String> {
    std::fs::read_to_string(format!("{GOLDEN}/rust/buyin/{path}")).ok()
}

#[test]
fn test_adapters_match_the_golden_file() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    golden("buyin.adapters.rs", &source);
}

#[test]
fn test_adapters_name_each_policy_by_its_domains() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    for name in [
        "#[policy(name = \"policy-buyin-wallet\", source = BuyIn, target = Wallet)]",
        "#[policy(name = \"policy-wallet-buyin\", source = Wallet, target = BuyIn)]",
    ] {
        assert!(source.contains(name), "{name}:\n{source}");
    }
}

#[test]
fn test_adapters_wire_the_refusal_callback_the_interface_declares() {
    let translated = buyin(&[translate_hold()]);
    let interface = |path: &str| {
        translated
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.source.clone())
    };
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let imp = inherent_impl(&file, "BuyInWallet");
    assert_eq!(
        markers(imp),
        s(&[("on_event", "BuyInStarted"), ("on_refused", "Hold")])
    );
    // The callback takes the refused command's context and answers a
    // Result, and forwards both: the interface's method reads the context
    // and answers the Result, its error the domain's `Bug`.
    let callback = methods(imp)[1];
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
        "->::core::result::Result<HoldLost,Bug>"
    );
    assert_eq!(
        text(&callback.block),
        "{crate::buy_in_wallet::BuyInWallet::hold_refused(&self.0,refusal.command(),refusal.refusal::<InsufficientFunds>(),ctx,)}"
    );
}

/// A command refusing nothing has no typed refusal to hand over: its
/// `<command>_refused` method takes the command and the context alone.
#[test]
fn test_a_refused_method_of_a_command_refusing_nothing_takes_no_refusal() {
    let deposit = Translate {
        command: "buyin.v1.Deposit".to_string(),
        ..translate_hold()
    };
    let files = buyin(&[deposit]);
    let source = &file_at(&files, "buy_in_wallet.rs").source;
    assert!(
        source.contains(
            "    fn deposit_refused(&self, command: &Deposit, ctx: &CommandContext) -> Result<HoldLost, Bug>;"
        ),
        "{source}"
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
    let files = scaffold(&descriptors(KINDS), KINDS, &[translate_audit()]).unwrap();
    let source = &file_at(&files, "ledger_audit_policy.rs").source;
    assert!(
        source.contains(
            "    fn audit_refused(\n        &self,\n        command: &Audit,\n        refusal: Option<AuditRefuses>,\n        ctx: &CommandContext,\n    ) -> Result<Arrived, Bug>;"
        ),
        "{source}"
    );
}

/// The adapter hands that method the refusal as the choice: each of the
/// command's refusal types tried in turn, the one it is chosen.
#[test]
fn test_adapters_hand_a_refused_method_the_choice_of_several_refusal_types() {
    let files = scaffold(&descriptors(KINDS), KINDS, &[translate_audit()]).unwrap();
    let interface = |path: &str| {
        files
            .iter()
            .find(|f| f.path == path)
            .map(|f| f.source.clone())
    };
    let source = adapters(&descriptors(KINDS), KINDS, &[], &interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let imp = inherent_impl(&file, "LedgerAuditPolicy");
    assert_eq!(
        markers(imp),
        s(&[("on_event", "Noted"), ("on_refused", "Audit")])
    );
    assert_eq!(
        text(&methods(imp)[1].block),
        "{crate::ledger_audit_policy::LedgerAuditPolicy::audit_refused(&self.0,refusal.command(),\
         refusal.refusal::<PostRefused>().map(AuditRefuses::PostRefused)\
         .or_else(||{refusal.refusal::<Unreasoned>().map(AuditRefuses::Unreasoned)}),ctx,)}"
    );
}

/// A `<command>_refused` method with no typed refusal takes the context
/// second: the parser does not take it for the refusal.
#[test]
fn test_adapters_forward_the_context_to_a_refused_method_with_no_typed_refusal() {
    let interface = |path: &str| {
        (path == "buy_in_wallet.rs").then(|| {
            "pub trait BuyInWallet {\n\
             fn hold_refused(&self, command: &Hold, ctx: &CommandContext) \
             -> Result<HoldLost, Bug>;\n}\n"
                .to_string()
        })
    };
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let imp = inherent_impl(&file, "BuyInWallet");
    assert_eq!(
        text(&methods(imp)[1].block),
        "{crate::buy_in_wallet::BuyInWallet::hold_refused(&self.0,refusal.command(),ctx)}"
    );
}

/// A fact handler's adapter hands the interface its context too, with no
/// Result: a fact is recorded, never refused.
#[test]
fn test_adapters_forward_a_fact_handlers_context() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let buy_in = inherent_impl(&file, "BuyIn");
    let fact = methods(buy_in)
        .into_iter()
        .find(|m| m.sig.ident == "hold_lost")
        .expect("the HoldLost handler");
    assert!(
        text(&fact.block).contains("crate::buy_in::BuyIn::hold_lost(&self.0,&fact,ctx)"),
        "{}",
        text(&fact.block)
    );
}

/// A command handler's adapter hands the interface its context; a policy
/// reaction's hands it its context and answers its Result.
#[test]
fn test_adapters_forward_each_handlers_context() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let wallet = inherent_impl(&file, "Wallet");
    assert_eq!(
        text(&methods(wallet)[0].block),
        "{crate::wallet::Wallet::deposit(&self.0,&cmd,ctx).map(Emit::of)}"
    );
    let policy = inherent_impl(&file, "BuyInWallet");
    let reaction = methods(policy)[0];
    assert_eq!(
        text(&reaction.sig.output),
        "->::core::result::Result<Commands<Wallet>,Bug>"
    );
    assert!(
        text(&reaction.block).starts_with(
            "{letorders=crate::buy_in_wallet::BuyInWallet::buy_in_started(&self.0,&ctx.source_root().value,event,ctx,)?;"
        ),
        "{}",
        text(&reaction.block)
    );
}

/// A policy reaction takes the source root, the event and its context, and
/// answers its commands or its error, a bug.
#[test]
fn test_scaffold_gives_a_policy_reaction_its_context_and_a_result() {
    let files = buyin(&[]);
    let source = &file_at(&files, "buy_in_wallet.rs").source;
    assert!(
        source.contains(
            "    fn buy_in_started(\n        &self,\n        source: &[u8],\n        event: &BuyInStarted,\n        ctx: &PolicyContext,\n    ) -> Result<Vec<(Vec<u8>, WalletCommand)>, Bug>;"
        ),
        "{source}"
    );
}

/// A domain's handlers fail with the generated layer's `Failed`, larger
/// than clippy's `result_large_err` allows; the generated impl allows it,
/// so the application need not.
#[test]
fn test_adapters_allow_a_domains_large_failure() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
    for ty in ["Wallet", "BuyIn"] {
        let imp = inherent_impl(&file, ty);
        assert!(
            imp.attrs
                .iter()
                .any(|a| text(a) == "#[allow(clippy::result_large_err)]"),
            "{ty}: {source}"
        );
    }
}

/// A policy a child process serves has no adapter here, named by its name
/// in the application; the domains it joins keep theirs.
#[test]
fn test_adapters_leave_a_child_policy_to_its_process() {
    let all = adapters(&descriptors(BUYIN), BUYIN, &[], &golden_interface).unwrap();
    assert!(all.contains("policy-buyin-wallet"), "{all}");
    let source = adapters(
        &descriptors(BUYIN),
        BUYIN,
        &["policy-buyin-wallet"],
        &golden_interface,
    )
    .unwrap();
    assert!(!source.contains("policy-buyin-wallet"), "{source}");
    assert!(source.contains("policy-wallet-buyin"), "{source}");
}

/// A domain served by a child process is no aggregate here: a policy
/// names it by its command set (`CommandTarget`, with `Decides` of each
/// command) and its event set (`EventSource`, with `Ingests` of each
/// fact), and no stand-in markup is generated for it.
#[test]
fn test_adapters_name_a_remote_domain_by_its_commands_and_events() {
    let source = adapters(&descriptors(BUYIN), BUYIN, &["buyin"], &golden_interface).unwrap();
    let file = syn::parse_file(&source).unwrap();
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
    let file = syn::parse_file(source).unwrap();
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
/// reaches it, `Ingests` only when a fact does: the wallet, commands alone,
/// imports `Decides` and no `Ingests`; the buy-in, a fact as well, both.
#[test]
fn test_adapters_import_a_remote_domains_markup_by_its_inputs() {
    let wallet = adapters(&descriptors(BUYIN), BUYIN, &["wallet"], &golden_interface).unwrap();
    let imports = markup_imports(&wallet);
    assert!(imports.iter().any(|m| m == "Decides"), "{imports:?}");
    assert!(!imports.iter().any(|m| m == "Ingests"), "{imports:?}");
    let buyin = adapters(&descriptors(BUYIN), BUYIN, &["buyin"], &golden_interface).unwrap();
    let imports = markup_imports(&buyin);
    assert!(imports.iter().any(|m| m == "Decides"), "{imports:?}");
    assert!(imports.iter().any(|m| m == "Ingests"), "{imports:?}");
}

// ------------------------------------------------------------- --missing

/// The wallet's interface as a user has it: Hold and Held absent (the proto
/// gained them later), a method and a doc comment of the user's own.
const USER_WALLET: &str = r#"//! The wallet.

use crate::domain::*;

/// What a wallet decides.
pub trait Wallet {
    /// The user's own doc, which `--missing` must keep byte for byte.
    fn deposit(&self, cmd: &Deposit) -> Result<Deposited, Rejection>;

    fn apply_deposited(&mut self, event: &Deposited); // USER-WRITTEN
}
"#;

const USER_WALLET_IMPL: &str = r#"//! The wallet, mine.

use crate::domain::*;
use crate::wallet::Wallet;

#[derive(Clone, Debug, Default)]
pub struct WalletImpl {
    pub state: crate::wire::WalletState,
}

impl Wallet for WalletImpl {
    fn deposit(&self, cmd: &Deposit) -> Result<Deposited, Rejection> {
        Ok(Deposited { cents: cmd.cents }) // USER-WRITTEN
    }

    fn apply_deposited(&mut self, event: &Deposited) {
        self.state.balance += event.cents;
    }
}
"#;

#[test]
fn test_complete_adds_only_the_absent_methods_to_the_interface() {
    let files = buyin(&[]);
    let completed = complete(file_at(&files, "wallet.rs"), USER_WALLET)
        .unwrap()
        .expect("hold and apply_held are absent");
    let expected = USER_WALLET.replace(
        "// USER-WRITTEN\n}",
        "// USER-WRITTEN\n\n    fn hold(&self, cmd: &Hold, ctx: &Context) -> Result<Held, Failed<InsufficientFunds>>;\n\n    \
         fn apply_held(&mut self, event: &Held);\n}",
    );
    assert_eq!(completed, expected);
}

#[test]
fn test_complete_adds_only_the_absent_methods_to_the_implementation() {
    let files = buyin(&[]);
    let completed = complete(file_at(&files, "wallet_impl.rs"), USER_WALLET_IMPL)
        .unwrap()
        .expect("hold and apply_held are absent");
    let expected = USER_WALLET_IMPL.replace(
        "        self.state.balance += event.cents;\n    }\n}",
        "        self.state.balance += event.cents;\n    }\n\n    \
         fn hold(&self, _cmd: &Hold, _ctx: &Context) -> Result<Held, Failed<InsufficientFunds>> {\n        todo!()\n    }\n\n    \
         fn apply_held(&mut self, _event: &Held) {\n        todo!()\n    }\n}",
    );
    assert_eq!(completed, expected);
}

#[test]
fn test_complete_of_a_complete_file_changes_nothing() {
    for f in buyin(&[translate_hold()]) {
        assert_eq!(complete(&f, &f.source), Ok(None), "{}", f.path);
    }
}

#[test]
fn test_complete_adds_a_newly_translated_refused_method_to_a_policy() {
    let before = buyin(&[]);
    let after = buyin(&[translate_hold()]);
    for path in ["buy_in_wallet.rs", "buy_in_wallet_impl.rs"] {
        let completed = complete(file_at(&after, path), &file_at(&before, path).source)
            .unwrap()
            .unwrap_or_else(|| panic!("{path}: the refused method is absent"));
        assert!(completed.contains("fn hold_refused("), "{completed}");
    }
}

#[test]
fn test_complete_refuses_a_fact_taken_as_a_command() {
    let files = buyin(&[]);
    let fresh = file_at(&files, "buy_in.rs");
    let as_command = fresh.source.replace("fact: &HoldLost", "cmd: &HoldLost");
    assert_ne!(as_command, fresh.source);
    assert_eq!(
        complete(fresh, &as_command),
        Err(CodegenError::InputMismatch {
            path: "buy_in.rs".to_string(),
            message: "buyin.v1.HoldLost".to_string(),
            expected: "fact".to_string(),
            found: "cmd".to_string(),
        })
    );
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
        complete(fresh, &as_fact),
        Err(CodegenError::InputMismatch {
            path: "buy_in_impl.rs".to_string(),
            message: "buyin.v1.RecordHeld".to_string(),
            expected: "cmd".to_string(),
            found: "fact".to_string(),
        })
    );
}

#[test]
fn test_complete_refuses_a_file_without_the_components_trait() {
    let files = buyin(&[]);
    assert_eq!(
        complete(
            file_at(&files, "wallet.rs"),
            "//! Not the wallet any more.\n"
        ),
        Err(CodegenError::ScaffoldImplNotFound {
            path: "wallet.rs".to_string(),
            component: "buyin.v1.WalletState".to_string(),
        })
    );
}

#[test]
fn test_complete_inserts_before_a_closing_brace_that_ends_a_line() {
    let files = buyin(&[]);
    let user = USER_WALLET.replace(
        "event: &Deposited); // USER-WRITTEN\n}\n",
        "event: &Deposited); }\n",
    );
    assert_ne!(user, USER_WALLET);
    let completed = complete(file_at(&files, "wallet.rs"), &user)
        .unwrap()
        .expect("hold and apply_held are absent");
    let expected = user.replace(
        "event: &Deposited); }\n",
        "event: &Deposited); \n\n    fn hold(&self, cmd: &Hold, ctx: &Context) -> Result<Held, Failed<InsufficientFunds>>;\n\n    \
         fn apply_held(&mut self, event: &Held);\n}\n",
    );
    assert_eq!(completed, expected);
}

#[test]
fn test_complete_takes_a_method_naming_its_event_otherwise_as_present() {
    let files = buyin(&[]);
    let fresh = file_at(&files, "wallet.rs");
    let renamed = fresh
        .source
        .replace("event: &Deposited", "deposited: &Deposited");
    assert_ne!(renamed, fresh.source);
    assert_eq!(complete(fresh, &renamed), Ok(None));
}
