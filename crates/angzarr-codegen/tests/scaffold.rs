//! The scaffold: from a project's protos, each component's markup impl for
//! angzarr-embedded-macros, written once into the user's source tree (`angzarr
//! scaffold rust`), and completed, never overwritten (`--missing`). Read
//! back as Rust items; that the scaffold compiles against angzarr-embedded is the
//! end-to-end test's (angzarr-build's `tests/local.rs`).

mod common;

use angzarr_codegen::{CodegenError, ScaffoldFile, Translate, complete, scaffold};
use common::*;

/// The buy-in project's scaffold, with `translate`.
fn buyin(translate: &[Translate]) -> Vec<ScaffoldFile> {
    scaffold(&descriptors(BUYIN), BUYIN, translate).unwrap()
}

/// The scaffold file for `component`, read back.
fn file_of(files: &[ScaffoldFile], component: &str) -> (ScaffoldFile, syn::File) {
    let file = files
        .iter()
        .find(|f| f.component == component)
        .unwrap_or_else(|| panic!("no scaffold for {component}: {files:#?}"))
        .clone();
    let parsed = syn::parse_file(&file.source).expect("the scaffold parses");
    (file, parsed)
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

/// Whether every method body is `todo!()`, and nothing else.
fn bodies_are_todo(imp: &syn::ItemImpl) -> bool {
    methods(imp).iter().all(|f| text(&f.block) == "{todo!()}")
}

#[test]
fn test_scaffold_writes_one_file_per_component_named_for_it() {
    let files = buyin(&[]);
    let mut placed: Vec<(String, String)> = files
        .iter()
        .map(|f| (f.component.clone(), f.path.clone()))
        .collect();
    placed.sort();
    assert_eq!(
        placed,
        s(&[
            ("buyin.v1.BuyInState", "buy_in.rs"),
            ("buyin.v1.BuyInWalletPolicy", "buy_in_wallet.rs"),
            ("buyin.v1.WalletBuyInPolicy", "wallet_buy_in.rs"),
            ("buyin.v1.WalletState", "wallet.rs"),
        ])
    );
}

#[test]
fn test_scaffold_writes_an_aggregate_with_a_stub_per_command_and_event() {
    let (_, file) = file_of(&buyin(&[]), "buyin.v1.WalletState");
    let wallet = item_struct_named(&file, "Wallet");
    assert!(
        derives(wallet).contains(&"Default".to_string()),
        "the registry builds every fresh instance from Default"
    );
    let imp = inherent_impl(&file, "Wallet");
    assert_eq!(
        impl_markup(imp),
        ("aggregate".to_string(), "domain=\"wallet\"".to_string())
    );
    assert_eq!(
        markers(imp),
        s(&[
            ("handles_cmd", "Deposit"),
            ("handles_cmd", "Hold,refuses(InsufficientFunds)"),
            ("applies", "Deposited"),
            ("applies", "Held"),
        ])
    );
    assert_eq!(
        signatures(imp),
        [
            "fndeposit(&self,cmd:Deposit,ctx:&Context)->Result<Deposited,Infallible>",
            "fnhold(&self,cmd:Hold,ctx:&Context)->Result<Held,Refuses<Self,Hold>>",
            "fndeposited(&mutself,event:Deposited)",
            "fnheld(&mutself,event:Held)",
        ]
    );
    assert!(bodies_are_todo(imp), "every body is todo!()");
}

#[test]
fn test_scaffold_marks_a_process_managers_starting_command() {
    let (_, file) = file_of(&buyin(&[]), "buyin.v1.BuyInState");
    let imp = inherent_impl(&file, "BuyIn");
    assert_eq!(
        impl_markup(imp),
        (
            "process_manager".to_string(),
            "domain=\"buyin\"".to_string()
        )
    );
    assert_eq!(
        markers(imp),
        s(&[
            ("handles_cmd", "StartBuyIn,starts"),
            ("handles_cmd", "RecordHeld"),
            ("handles_fact", "HoldLost"),
            ("applies", "BuyInStarted"),
            ("applies", "BuyInHeld"),
            ("applies", "BuyInFailed"),
        ])
    );
    assert_eq!(
        signatures(imp)[..3],
        [
            "fnstart_buy_in(&self,cmd:StartBuyIn,ctx:&Context)->Result<BuyInStarted,Infallible>",
            "fnrecord_held(&self,cmd:RecordHeld,ctx:&Context)->Result<BuyInHeld,Infallible>",
            "fnhold_lost(&self,fact:HoldLost,ctx:&Context)->BuyInFailed",
        ]
    );
    assert!(bodies_are_todo(imp), "every body is todo!()");
}

#[test]
fn test_scaffold_writes_a_policy_with_a_stub_per_event_it_consumes() {
    let (_, file) = file_of(&buyin(&[]), "buyin.v1.BuyInWalletPolicy");
    let policy = item_struct_named(&file, "BuyInWallet");
    assert!(
        matches!(policy.fields, syn::Fields::Unit),
        "a policy is stateless: a unit struct"
    );
    let imp = inherent_impl(&file, "BuyInWallet");
    assert_eq!(
        impl_markup(imp),
        (
            "policy".to_string(),
            "name=\"buy-in-wallet\",source=BuyIn,target=Wallet".to_string()
        )
    );
    assert_eq!(markers(imp), s(&[("on_event", "BuyInStarted")]));
    assert_eq!(
        signatures(imp),
        ["fnbuy_in_started(&self,event:&BuyInStarted,ctx:&PolicyContext)->Commands<Wallet>"]
    );
    assert!(bodies_are_todo(imp), "every body is todo!()");
}

#[test]
fn test_scaffold_passes_refusals_through_by_default() {
    let files = buyin(&[]);
    for policy in ["buyin.v1.BuyInWalletPolicy", "buyin.v1.WalletBuyInPolicy"] {
        let (scaffolded, file) = file_of(&files, policy);
        let imp = file
            .items
            .iter()
            .find_map(|i| match i {
                syn::Item::Impl(imp) if imp.trait_.is_none() => Some(imp),
                _ => None,
            })
            .unwrap();
        assert!(
            !markers(imp).iter().any(|(name, _)| name == "on_refused"),
            "{}: no on_refused callback without the switch",
            scaffolded.path
        );
    }
}

#[test]
fn test_scaffold_writes_an_on_refused_callback_stub_for_each_command_the_switch_translates() {
    let (_, file) = file_of(&buyin(&[translate_hold()]), "buyin.v1.BuyInWalletPolicy");
    let imp = inherent_impl(&file, "BuyInWallet");
    assert_eq!(
        markers(imp),
        s(&[("on_event", "BuyInStarted"), ("on_refused", "Hold")])
    );
    assert_eq!(
        signatures(imp)[1],
        "fnhold_refused(&self,refusal:&Refusal<Hold>)->HoldLost"
    );
    assert!(bodies_are_todo(imp), "every body is todo!()");
}

#[test]
fn test_scaffold_translate_touches_only_the_policy_it_names() {
    let (_, file) = file_of(&buyin(&[translate_hold()]), "buyin.v1.WalletBuyInPolicy");
    let imp = inherent_impl(&file, "WalletBuyIn");
    assert_eq!(markers(imp), s(&[("on_event", "Held")]));
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

// ------------------------------------------------------------- --missing

/// The wallet's scaffold as a user has it: Deposit's body written, Hold's
/// handler and Held's applier not there yet (the proto gained them later).
const USER_WALLET: &str = r#"//! The wallet.

use std::convert::Infallible;

use angzarr_embedded::next::markup::{Context, Refuses};
use angzarr_embedded_macros::{aggregate, applies, handles_cmd};

use crate::domain::*;

#[derive(Clone, Debug, Default)]
pub struct Wallet {
    balance: i64,
}

#[aggregate(domain = "wallet")]
impl Wallet {
    /// The user's own body, which `--missing` must keep byte for byte.
    #[handles_cmd(Deposit)]
    fn deposit(&self, cmd: Deposit, _ctx: &Context) -> Result<Deposited, Infallible> {
        Ok(Deposited { cents: cmd.cents }) // USER-WRITTEN
    }

    #[applies(Deposited)]
    fn deposited(&mut self, event: Deposited) {
        self.balance += event.cents;
    }
}
"#;

fn wallet_scaffold() -> ScaffoldFile {
    buyin(&[])
        .into_iter()
        .find(|f| f.component == "buyin.v1.WalletState")
        .unwrap()
}

#[test]
fn test_complete_adds_only_the_absent_stubs() {
    let completed = complete(&wallet_scaffold(), USER_WALLET)
        .unwrap()
        .expect("Hold and Held are absent");
    let file = syn::parse_file(&completed).expect("the completed file parses");
    let imp = inherent_impl(&file, "Wallet");
    assert_eq!(
        markers(imp),
        s(&[
            ("handles_cmd", "Deposit"),
            ("applies", "Deposited"),
            ("handles_cmd", "Hold,refuses(InsufficientFunds)"),
            ("applies", "Held"),
        ])
    );
}

#[test]
fn test_complete_never_changes_what_the_user_wrote() {
    let completed = complete(&wallet_scaffold(), USER_WALLET).unwrap().unwrap();
    // The user's text is the completed text with one contiguous run of
    // inserted stubs taken out.
    let prefix = USER_WALLET
        .bytes()
        .zip(completed.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let rest = &USER_WALLET[prefix..];
    assert!(
        completed.len() > USER_WALLET.len() && completed.ends_with(rest),
        "the user's text survives byte for byte around one insertion:\n{completed}"
    );
}

#[test]
fn test_complete_of_a_complete_file_changes_nothing() {
    let fresh = wallet_scaffold();
    assert_eq!(complete(&fresh, &fresh.source), Ok(None));
}

/// The buy-in's fresh scaffold.
fn buy_in_scaffold() -> ScaffoldFile {
    buyin(&[])
        .into_iter()
        .find(|f| f.component == "buyin.v1.BuyInState")
        .unwrap()
}

#[test]
fn test_scaffold_writes_handles_fact_for_a_fact_and_handles_cmd_for_a_command() {
    // HoldLost is declared `fact: true`; the buy-in's other inputs are
    // commands. A fact's handler returns no Result and refuses nothing.
    let file = syn::parse_file(&buy_in_scaffold().source).unwrap();
    let imp = inherent_impl(&file, "BuyIn");
    let inputs: Vec<(String, String)> = markers(imp)
        .into_iter()
        .filter(|(name, _)| name.starts_with("handles_"))
        .collect();
    assert_eq!(
        inputs,
        s(&[
            ("handles_cmd", "StartBuyIn,starts"),
            ("handles_cmd", "RecordHeld"),
            ("handles_fact", "HoldLost"),
        ])
    );
    let fact = methods(imp)
        .into_iter()
        .find(|f| f.sig.ident == "hold_lost")
        .expect("hold_lost");
    assert!(
        !text(&fact.sig.output).contains("Result"),
        "a fact's handler is infallible: {}",
        text(&fact.sig.output)
    );
}

#[test]
fn test_complete_refuses_handles_cmd_on_a_fact() {
    let fresh = buy_in_scaffold();
    let as_command = fresh
        .source
        .replace("handles_fact(HoldLost)", "handles_cmd(HoldLost)");
    assert_ne!(as_command, fresh.source, "the scaffold handles HoldLost");
    assert_eq!(
        complete(&fresh, &as_command),
        Err(CodegenError::MarkerMismatch {
            path: "buy_in.rs".to_string(),
            message: "buyin.v1.HoldLost".to_string(),
            expected: "handles_fact".to_string(),
            found: "handles_cmd".to_string(),
        })
    );
}

#[test]
fn test_complete_refuses_handles_fact_on_a_command() {
    let fresh = buy_in_scaffold();
    let as_fact = fresh
        .source
        .replace("handles_cmd(RecordHeld)", "handles_fact(RecordHeld)");
    assert_ne!(as_fact, fresh.source, "the scaffold handles RecordHeld");
    assert_eq!(
        complete(&fresh, &as_fact),
        Err(CodegenError::MarkerMismatch {
            path: "buy_in.rs".to_string(),
            message: "buyin.v1.RecordHeld".to_string(),
            expected: "handles_cmd".to_string(),
            found: "handles_fact".to_string(),
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
fn test_complete_adds_a_newly_translated_callback_to_a_policy() {
    let files = buyin(&[]);
    let (before, _) = file_of(&files, "buyin.v1.BuyInWalletPolicy");
    let (after, _) = file_of(&buyin(&[translate_hold()]), "buyin.v1.BuyInWalletPolicy");
    let completed = complete(&after, &before.source)
        .unwrap()
        .expect("the callback is absent");
    let file = syn::parse_file(&completed).unwrap();
    assert_eq!(
        markers(inherent_impl(&file, "BuyInWallet")),
        s(&[("on_event", "BuyInStarted"), ("on_refused", "Hold")])
    );
}

#[test]
fn test_complete_refuses_a_file_without_the_components_impl() {
    let fresh = wallet_scaffold();
    assert_eq!(
        complete(&fresh, "//! Not the wallet any more.\n"),
        Err(CodegenError::ScaffoldImplNotFound {
            path: "wallet.rs".to_string(),
            component: "buyin.v1.WalletState".to_string(),
        })
    );
}

// --------------------------------------------------------------- helpers

fn item_struct_named<'a>(file: &'a syn::File, name: &str) -> &'a syn::ItemStruct {
    file.items
        .iter()
        .find_map(|i| match i {
            syn::Item::Struct(st) if st.ident == name => Some(st),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no struct {name}"))
}

/// The traits a struct derives.
fn derives(item: &syn::ItemStruct) -> Vec<String> {
    item.attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .flat_map(|a| {
            a.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .unwrap()
        })
        .map(text)
        .collect()
}

#[test]
fn test_scaffold_skips_a_projector_beside_the_markup() {
    let files = scaffold(&descriptors(BLACKJACK), BLACKJACK, &[]).expect("a projector is skipped");
    assert!(
        !files
            .iter()
            .any(|f| f.component == "io.angzarr.examples.v1.LedgerProjection"),
        "{:?}",
        files.iter().map(|f| &f.component).collect::<Vec<_>>()
    );
    assert!(
        files
            .iter()
            .any(|f| f.component == "io.angzarr.examples.v1.TableState")
    );
}
