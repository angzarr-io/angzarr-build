//! The Python scaffold of every component kind (decision 14.50): a
//! projector over several source domains with an async logic, a policy, a
//! process manager with a fact, typed refusals (several to a command, the
//! domain's rejection codes as a Reason enum) and each emits cardinality,
//! the component's name from the proto. Each generated file sits behind
//! angzarr-client-python's public API, so a hand-written one replaces it:
//! the generated code imports nothing else of the client.

mod common;

use angzarr_codegen::{Owner, ProtoInput, PythonApp, ScaffoldFile, refusal_key, scaffold_python};
use common::*;

const LEDGER: &str = "io/angzarr/examples/v1/ledger.proto";
const POLICIES: &str = "io/angzarr/examples/v1/policies.proto";
const BUY_IN: &str = "io/angzarr/examples/v1/buy_in.proto";
const TABLE: &str = "io/angzarr/examples/v1/table.proto";
const TILL: &[&str] = &["pyshop/v1/till.proto"];

fn app(domain: &str) -> PythonApp {
    PythonApp {
        domain: domain.to_string(),
        protos: ProtoInput::Directory("proto".to_string()),
        rules_version: "0.1.0".to_string(),
    }
}

fn blackjack(file: &str, domain: &str) -> Vec<ScaffoldFile> {
    scaffold_python(&descriptors(BLACKJACK), &[file], &app(domain))
        .unwrap_or_else(|e| panic!("{file} as {domain}: {e:?}"))
}

fn source<'a>(files: &'a [ScaffoldFile], suffix: &str) -> &'a str {
    &files
        .iter()
        .find(|f| f.path.ends_with(suffix))
        .unwrap_or_else(|| {
            panic!(
                "no {suffix}: {:?}",
                files.iter().map(|f| &f.path).collect::<Vec<_>>()
            )
        })
        .source
}

/// `text` with every run of whitespace one space and black's line
/// splitting undone (`(\n    a,\n)` as `(a)`), so a pin holds whatever
/// the width puts on a line.
fn flat(text: &str) -> String {
    let spaced = text.split_whitespace().collect::<Vec<_>>().join(" ");
    spaced
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(",)", ")")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace(",}", "}")
}

fn assert_has(text: &str, expected: &[&str]) {
    let flattened = flat(text);
    for e in expected {
        assert!(flattened.contains(&flat(e)), "{e}:\n{text}");
    }
}

// ------------------------------------------------------------ projector

/// The ledger: one projector over the table and the wallet
/// (`input_domains`), named by its `component_name`.
fn ledger() -> Vec<ScaffoldFile> {
    blackjack(LEDGER, "projector-wallet-table-ledger")
}

#[test]
fn test_a_projector_is_scaffolded_under_its_component_name() {
    let files = ledger();
    let package = "src/angzarr_projector_wallet_table_ledger";
    for (path, owner) in [
        (format!("{package}/logic.py"), Owner::User),
        (format!("{package}/logic_impl.py"), Owner::User),
        (format!("{package}/domain.py"), Owner::Generated),
        (format!("{package}/main.py"), Owner::Generated),
        (
            format!("{package}/_gen/io/angzarr/examples/v1/ledger_projector_angzarr.py"),
            Owner::Generated,
        ),
    ] {
        let found = files.iter().find(|f| f.path == path);
        assert_eq!(found.map(|f| f.owner), Some(owner), "{path}");
    }
}

/// Its logic is async: one coroutine per projected event, of either
/// source domain, taking the projection and the event in the domain's
/// plain types and returning the next projection.
#[test]
fn test_a_projectors_logic_projects_each_event_of_every_source_asynchronously() {
    let files = ledger();
    let logic = source(&files, "/logic.py");
    assert!(logic.contains("    PageContext,\n"), "{logic}");
    assert!(!logic.contains("CommandContext"), "{logic}");
    assert_has(
        logic,
        &[
            "class LedgerProjectorLogic(Protocol):",
            "async def project_round_settled(self, projection: LedgerProjection, \
             event: RoundSettled, ctx: PageContext) -> LedgerProjection: ...",
            "async def project_funds_held(self, projection: LedgerProjection, \
             event: FundsHeld, ctx: PageContext) -> LedgerProjection: ...",
        ],
    );
    let implementation = source(&files, "/logic_impl.py");
    assert_has(
        implementation,
        &[
            "class LedgerProjectorLogicImpl(LedgerProjectorLogic):",
            "async def project_round_settled(self, projection: LedgerProjection, \
             event: RoundSettled, ctx: PageContext) -> LedgerProjection: \
             raise NotImplementedError # TODO: project RoundSettled",
        ],
    );
}

/// The dispatch is the client's public `ProjectorDispatch`, named for the
/// component, folding books of both sources; each event's coroutine runs
/// to its end on the dispatch's one event loop.
#[test]
fn test_a_projectors_dispatch_folds_both_sources_through_its_coroutines() {
    let files = ledger();
    let dispatch = source(&files, "ledger_projector_angzarr.py");
    assert_has(
        dispatch,
        &[
            "def new_ledger_projector_dispatch(logic: LedgerProjectorLogic) -> _az.ProjectorDispatch:",
            "dispatch = _az.ProjectorDispatch(\"projector-wallet-table-ledger\", _ledger.LedgerProjection)",
            "dispatch.for_domains(\"table\", \"wallet\")",
            "projected = _run(logic.project_round_settled(_to_ledger_projection(projection), event, ctx))",
            "projection.CopyFrom(_from_ledger_projection(projected))",
            "dispatch.on_event_with_context(\"io.angzarr.examples.v1.RoundSettled\", _project_round_settled)",
            "dispatch.on_event_with_context(\"io.angzarr.examples.v1.FundsHeld\", _project_funds_held)",
            "return asyncio.run_coroutine_threadsafe(coroutine, _LOOP).result()",
        ],
    );
    let main = source(&files, "/main.py");
    assert_has(
        main,
        &[
            "host.add_projector(new_ledger_projector_dispatch(LedgerProjectorLogicImpl()), DESCRIPTOR)",
        ],
    );
}

/// A map field is a dict of the plain type, converted entry by entry.
#[test]
fn test_a_map_field_is_a_dict_of_its_plain_type() {
    let files = ledger();
    let domain = source(&files, "/domain.py");
    assert_has(
        domain,
        &[
            "from dataclasses import dataclass, field",
            "wallets: dict[str, WalletLedgerRow] = field(default_factory=dict)",
            "@dataclass(frozen=True) class WalletLedgerRow:",
            "from angzarr_client.router import PageContext as PageContext",
        ],
    );
    assert!(!domain.contains("CommandContext"), "{domain}");
    // A map's entry is no plain type of its own.
    assert!(!domain.contains("Entry"), "{domain}");
    let dispatch = source(&files, "ledger_projector_angzarr.py");
    assert_has(
        dispatch,
        &[
            "wallets={k: _to_wallet_ledger_row(v) for k, v in pb.wallets.items()},",
            "wallets={k: _from_wallet_ledger_row(v) for k, v in value.wallets.items()},",
        ],
    );
}

/// A command declaring no emits returns raw payloads of any of its
/// domain's events (unset is MANY): a list of any of them, typed, as Rust's
/// `Vec<LedgerStateEvent>`, not nothing.
#[test]
fn test_a_command_with_no_emits_returns_a_list_of_any_of_its_domains_events() {
    let files = scaffold_python(
        &descriptors(&["kinds/v1/kinds.proto"]),
        &["kinds/v1/kinds.proto"],
        &app("ledger"),
    )
    .unwrap();
    assert_has(
        source(&files, "/logic.py"),
        &[
            "def raw(self, cmd: Raw, state: LedgerState, ctx: CommandContext) \
           -> list[Posted | Noted | Cleared | Closed]: ...",
        ],
    );
}

// --------------------------------------------------------------- policy

/// table -> player: each TurnStarted becomes the player's commands, each
/// its target root.
fn table_player() -> Vec<ScaffoldFile> {
    blackjack(POLICIES, "policy-table-player")
}

#[test]
fn test_a_policys_logic_reacts_to_each_event_with_its_targets_commands() {
    let files = table_player();
    let logic = source(&files, "/logic.py");
    assert_has(
        logic,
        &[
            "class TablePlayerPolicyLogic(Protocol):",
            "def turn_started(self, event: TurnStarted, source: bytes, ctx: PageContext) \
             -> list[tuple[bytes, DecideMove]]: ...",
        ],
    );
    for import in [
        "    DecideMove,\n",
        "    PageContext,\n",
        "    TurnStarted,\n",
    ] {
        assert!(logic.contains(import), "{import}:\n{logic}");
    }
    assert!(!logic.contains("TablePlayerPolicy,"), "{logic}");
    let domain = source(&files, "/domain.py");
    assert_has(
        domain,
        &[
            "@dataclass(frozen=True) class DecideMove:",
            "from angzarr_client.router import PageContext as PageContext",
        ],
    );
}

/// A type its events and its target's commands both reach (`Card`, its
/// `Suit`) is declared, and converted, once: the commands are read from
/// the same protos as the events.
#[test]
fn test_a_policys_shared_types_are_declared_and_converted_once() {
    let files = table_player();
    let domain = source(&files, "/domain.py");
    for class in ["class Card:", "class Suit(enum.IntEnum):"] {
        assert_eq!(domain.matches(class).count(), 1, "{class}:\n{domain}");
    }
    let dispatch = source(&files, "table_player_policy_angzarr.py");
    for converter in ["def _to_card(pb)", "def _from_card(value"] {
        assert_eq!(
            dispatch.matches(converter).count(),
            1,
            "{converter}:\n{dispatch}"
        );
    }
}

#[test]
fn test_a_policys_dispatch_addresses_each_command_to_its_target() {
    let files = table_player();
    let dispatch = source(&files, "table_player_policy_angzarr.py");
    // One parameter too long for a line: black's split, a trailing comma.
    assert!(
        dispatch.contains(
            "def new_table_player_policy_dispatch(\n    logic: TablePlayerPolicyLogic,\n) -> _az.PolicyDispatch:\n"
        ),
        "{dispatch}"
    );
    assert_has(
        dispatch,
        &[
            "def new_table_player_policy_dispatch(logic: TablePlayerPolicyLogic) -> _az.PolicyDispatch:",
            "dispatch = _az.PolicyDispatch(\"policy-table-player\", \"table\", \"player\")",
            "book = _t.CommandBook(cover=_t.Cover(domain=\"player\"))",
            "book.cover.root.value = root",
            "return _commands(logic.turn_started(event, source, ctx))",
            "dispatch.on_event_with_context(\"io.angzarr.examples.v1.TurnStarted\", _turn_started)",
        ],
    );
    let main = source(&files, "/main.py");
    assert_has(
        main,
        &[
            "host.add_policy(new_table_player_policy_dispatch(TablePlayerPolicyLogicImpl()), DESCRIPTOR)",
        ],
    );
}

// ------------------------------------------------------ process manager

fn buy_in() -> Vec<ScaffoldFile> {
    blackjack(BUY_IN, "buy-in")
}

/// The process manager is a domain: decisions and folds as an
/// aggregate's, its fact recorded as received and flagged by what it
/// emits, served as the client's `ProcessManagerDispatch`.
#[test]
fn test_a_process_manager_is_scaffolded_with_its_fact() {
    let files = buy_in();
    let logic = source(&files, "/logic.py");
    assert_has(
        logic,
        &[
            "class BuyInLogic(Protocol):",
            "def start_buy_in(self, cmd: StartBuyIn, state: BuyInState, ctx: CommandContext) -> BuyInStarted: ...",
            "def seat_confirmation_refused(self, fact: SeatConfirmationRefused, state: BuyInState, \
             ctx: CommandContext) -> BuyInSeatLost: ...",
        ],
    );
    let dispatch = source(&files, "buy_in_process_manager_angzarr.py");
    assert_has(
        dispatch,
        &[
            "dispatch = _az.ProcessManagerDispatch(\"pmg-buy-in\", \"buy-in\", rebuilder)",
            "return _az.FactRecord(wire, tuple(_wire(flags)))",
            "dispatch.on_fact(\"io.angzarr.examples.v1.SeatConfirmationRefused\", _on_seat_confirmation_refused)",
        ],
    );
    let main = source(&files, "/main.py");
    assert_has(
        main,
        &[
            "host.add_process_manager(new_buy_in_process_manager_dispatch(BuyInLogicImpl()), DESCRIPTOR)",
        ],
    );
    let domain = source(&files, "/domain.py");
    assert_has(domain, &["class BuyInStatePhase(enum.IntEnum):"]);
}

// ------------------------------------------------ refusals, cardinality

fn till() -> Vec<ScaffoldFile> {
    scaffold_python(&descriptors(TILL), TILL, &app("till")).unwrap()
}

/// A decision returns its events or one of its typed refusals, several to
/// a command; the return follows the emits cardinality: one event (a
/// fixed tuple for several), that or None, or a list of any of them.
#[test]
fn test_a_decision_returns_its_cardinality_or_one_of_its_refusals() {
    let files = till();
    let logic = source(&files, "/logic.py");
    assert_has(
        logic,
        &[
            "def charge(self, cmd: Charge, state: TillState, ctx: CommandContext) \
             -> Charged | TillRefused | CardDeclined: ...",
            "def refund(self, cmd: Refund, state: TillState, ctx: CommandContext) \
             -> Refunded | None | TillRefused: ...",
            "def settle(self, cmd: Settle, state: TillState, ctx: CommandContext) \
             -> list[Charged | Refunded]: ...",
            "def open(self, cmd: Open, state: TillState, ctx: CommandContext) \
             -> tuple[Charged, Refunded]: ...",
            "def close(self, cmd: Close, state: TillState, ctx: CommandContext) \
             -> tuple[Charged, Refunded] | None | Unbalanced: ...",
            "def cash_counted(self, fact: CashCounted, state: TillState, ctx: CommandContext) \
             -> Refunded: ...",
        ],
    );
}

/// The dispatch answers a returned refusal as the client's `Refused`, the
/// refusal on the wire.
#[test]
fn test_the_dispatch_refuses_with_a_returned_typed_refusal() {
    let files = till();
    let dispatch = source(&files, "till_state_angzarr.py");
    assert_has(
        dispatch,
        &[
            "_REFUSALS = {_d.TillRefused: _from_till_refused, _d.CardDeclined: _from_card_declined, _d.Unbalanced: _from_unbalanced,}",
            "to_wire = _REFUSALS.get(type(result))",
            "raise _az.Refused(to_wire(result))",
            "return _decided(logic.charge(cmd, _to_till_state(state), cctx))",
            // A command refusing nothing answers its events as before.
            "return _book(logic.settle(cmd, _to_till_state(state), cctx))",
        ],
    );
}

/// A refusal's codes are its Reason enum, each value by its name without
/// the enum's prefix (the ErrorInfo.reason on the wire); its diagnostic
/// key is the refusal's full name and its reason, its args its set fields.
#[test]
fn test_a_refusals_reason_enum_is_its_rejection_code() {
    let files = till();
    let domain = source(&files, "/domain.py");
    assert_has(
        domain,
        &[
            "class TillRefusedReason(enum.IntEnum): UNSPECIFIED = 0 TILL_CLOSED = 1 OVER_LIMIT = 2",
            "reason: TillRefusedReason = TillRefusedReason.UNSPECIFIED",
            "limit: int | None = None",
            "def key(self) -> str:",
            "return {TillRefusedReason.TILL_CLOSED: \"pyshop-v1-tillrefused-till-closed\", \
             TillRefusedReason.OVER_LIMIT: \"pyshop-v1-tillrefused-over-limit\",}\
             .get(self.reason, \"pyshop-v1-tillrefused\")",
            "def args(self) -> dict[str, object]:",
        ],
    );
    let out = run_domain(
        domain,
        "r = TillRefused(reason=TillRefusedReason.OVER_LIMIT, limit=5)\n\
         assert r.key() == 'pyshop-v1-tillrefused-over-limit', r.key()\n\
         assert r.args() == {'reason': 'OVER_LIMIT', 'limit': 5}, r.args()\n\
         assert TillRefused(reason=TillRefusedReason.TILL_CLOSED).args() == {'reason': 'TILL_CLOSED'}\n\
         d = CardDeclined(issuer='visa')\n\
         assert d.key() == 'pyshop-v1-carddeclined', d.key()\n\
         assert d.args() == {'issuer': 'visa'}, d.args()\n\
         assert CardDeclined().args() == {}\n\
         u = Unbalanced(reason='short')\n\
         assert u.key() == 'pyshop-v1-unbalanced', u.key()\n\
         assert u.args() == {'reason': 'short'}, u.args()\n\
         print('ok')\n",
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
}

/// A typed refusal's key is the same string in Python as in Rust
/// (`angzarr_codegen::refusal_key`, what the Rust domain layer's
/// `Diagnostic::code` is): a nested `Reason` held as `reason` keys by its
/// wire name, and a `reason` of any other enum (not nested, of another
/// name) keys as the refusal alone.
#[test]
fn test_a_refusals_key_is_the_same_in_python_as_in_rust() {
    const KINDS: &str = "kinds/v1/kinds.proto";
    let files = scaffold_python(&descriptors(&[KINDS]), &[KINDS], &app("ledger")).unwrap();
    let domain = source(&files, "/domain.py");
    let cases = [
        (
            "PostRefused(reason=PostRefusedReason.CLOSED)",
            refusal_key("kinds.v1.PostRefused", Some("CLOSED")),
        ),
        (
            "PostRefused(reason=PostRefusedReason.OVER_LIMIT)",
            refusal_key("kinds.v1.PostRefused", Some("OVER_LIMIT")),
        ),
        (
            "Unreasoned(reason=Tone.LOUD)",
            refusal_key("kinds.v1.Unreasoned", None),
        ),
        (
            "Mislabeled(reason=MislabeledCause.LATE)",
            refusal_key("kinds.v1.Mislabeled", None),
        ),
        (
            "Elsewhere(why=ElsewhereReason.AWAY)",
            refusal_key("kinds.v1.Elsewhere", None),
        ),
    ];
    let script: String = cases
        .iter()
        .map(|(value, key)| format!("assert {value}.key() == {key:?}, {value}.key()\n"))
        .chain(["print('ok')\n".to_string()])
        .collect();
    let out = run_domain(domain, &script);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "ok\n");
    let rust = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/kinds.domain.rs"
    ))
    .unwrap();
    for (_, key) in &cases {
        assert!(rust.contains(&format!("{key:?}")), "{key} in Rust");
    }
}

/// The spec's table: every command refusing the table's codes, ConfirmSeat
/// its own, the cardinalities table.proto declares.
#[test]
fn test_the_spec_table_is_scaffolded() {
    let files = blackjack(TABLE, "table");
    let logic = source(&files, "/logic.py");
    assert_has(
        logic,
        &[
            "def place_bet(self, cmd: PlaceBet, state: TableState, ctx: CommandContext) \
             -> BetPlaced | TableRefused: ...",
            "def confirm_seat(self, cmd: ConfirmSeat, state: TableState, ctx: CommandContext) \
             -> PlayerSeated | None | SeatNotConfirmable: ...",
            "def release_seat(self, cmd: ReleaseSeat, state: TableState, ctx: CommandContext) \
             -> SeatReleased | None: ...",
            "def hit(self, cmd: Hit, state: TableState, ctx: CommandContext) \
             -> list[CardDealt | TurnStarted | DealerPlayed | RoundSettled] | TableRefused: ...",
        ],
    );
    let dispatch = source(&files, "table_aggregate_angzarr.py");
    assert_has(
        dispatch,
        &["_az.AggregateDispatch(\"table\", \"table\", rebuilder)"],
    );
}

// --------------------------------------------------------------- names

/// The name an application knows a component by: its `component_name`,
/// else its kind's default (`policy-<input>-<output>`, `pmg-<domain>`,
/// an aggregate's domain), on its dispatch and its descriptor alike.
#[test]
fn test_each_component_takes_its_name_from_the_proto() {
    for (files, dispatch, name) in [
        (
            ledger(),
            "ledger_projector_angzarr.py",
            "projector-wallet-table-ledger",
        ),
        (
            table_player(),
            "table_player_policy_angzarr.py",
            "policy-table-player",
        ),
        (buy_in(), "buy_in_process_manager_angzarr.py", "pmg-buy-in"),
        (till(), "till_state_angzarr.py", "till"),
    ] {
        let source_ = source(&files, dispatch);
        assert!(
            flat(source_).contains(&format!("Dispatch(\"{name}\"")),
            "{name}:\n{source_}"
        );
        let descriptor = source(&files, "/_descriptor.py");
        let hex: String = descriptor
            .lines()
            .filter_map(|l| l.strip_prefix("        \"")?.strip_suffix('"'))
            .collect();
        let named = format!("0a{:02x}{}", name.len(), hex_of(name));
        assert!(hex.starts_with(&named), "{name}: {hex}");
    }
}

fn hex_of(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

/// A projector with no `component_name` is `projector-` and its anchor's
/// name in kebab case.
#[test]
fn test_a_projectors_default_name_is_its_anchor_in_kebab_case() {
    const FILE: &str = "pyshop/v1/views.proto";
    let files = scaffold_python(
        &descriptors(&[FILE]),
        &[FILE],
        &app("projector-http-server-view"),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let dispatch = source(&files, "_angzarr.py");
    assert!(
        flat(dispatch).contains("_az.ProjectorDispatch(\"projector-http-server-view\""),
        "{dispatch}"
    );
    // Its sources are its events' domains, in order.
    assert!(
        flat(dispatch).contains("dispatch.for_domains(\"pages\")"),
        "{dispatch}"
    );
}

// ------------------------------------------------- hand-replaceability

/// What the generated code may use of angzarr-client-python: its public
/// API only, so a hand-written file in its place is written against the
/// same. Each import of `angzarr_client`, and each attribute of the
/// router (`_az.`), is one of these.
const PUBLIC_IMPORTS: &[&str] = &[
    "from angzarr_client import ComponentHost, configure_logging",
    "from angzarr_client.child import run_child",
    "from angzarr_client.proto.io.angzarr.v1 import types_pb2 as _t",
    "import angzarr_client.router as _az",
    "from angzarr_client.router import Bug as Bug",
    "from angzarr_client.router import CommandContext as CommandContext",
    "from angzarr_client.router import PageContext as PageContext",
    "from angzarr_client.router import Rejection as Rejection",
];
/// `angzarr_client.router.__all__` at the pinned client.
const ROUTER_ALL: &[&str] = &[
    "WILDCARD_DOMAIN",
    "AggregateDispatch",
    "Bug",
    "CodedError",
    "CommandContext",
    "Conflict",
    "FactRecord",
    "GrpcCode",
    "Immediate",
    "PageContext",
    "PolicyDispatch",
    "ProcessManagerDispatch",
    "ProjectorDispatch",
    "Rebuilder",
    "Refused",
    "Rejection",
    "RetryableBug",
    "Router",
    "UnclassifiedError",
    "any_decode_error",
    "current_cover",
    "current_page",
    "pack",
];

#[test]
fn test_the_generated_code_uses_only_the_clients_public_api() {
    for files in [ledger(), table_player(), buy_in(), till()] {
        for f in files.iter().filter(|f| f.path.ends_with(".py")) {
            for line in f.source.lines() {
                let import = line.starts_with("from angzarr_client")
                    || line.starts_with("import angzarr_client");
                assert!(
                    !import || PUBLIC_IMPORTS.contains(&line),
                    "{}: {line}",
                    f.path
                );
            }
            for (at, _) in f.source.match_indices("_az.") {
                let name: String = f.source[at + 4..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    ROUTER_ALL.contains(&name.as_str()),
                    "{}: _az.{name}",
                    f.path
                );
            }
        }
    }
}

/// The user's files of every kind name nothing of the harness.
#[test]
fn test_the_users_files_of_every_kind_name_nothing_of_the_harness() {
    for files in [ledger(), table_player(), buy_in(), till()] {
        for f in files
            .iter()
            .filter(|f| f.owner == Owner::User && f.path.ends_with(".py"))
        {
            for harness in ["angzarr_client", "ComponentHost", "_az", "_pb2", "protobuf"] {
                assert!(!f.source.contains(harness), "{} names {harness}", f.path);
            }
        }
    }
}

/// Runs `script` with python3 beside `domain` (a generated `domain.py`),
/// an `angzarr_client.router` stub standing in for the client.
fn run_domain(domain: &str, script: &str) -> std::process::Output {
    static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "angzarr-codegen-python-components-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let client = dir.join("angzarr_client");
    std::fs::create_dir_all(&client).unwrap();
    std::fs::write(client.join("__init__.py"), "").unwrap();
    std::fs::write(
        client.join("router.py"),
        "class Bug(Exception): ...\nclass CommandContext: ...\nclass PageContext: ...\n\
         class Rejection(Exception): ...\n",
    )
    .unwrap();
    std::fs::write(dir.join("domain.py"), domain).unwrap();
    let main = dir.join("check.py");
    std::fs::write(
        &main,
        format!(
            "import sys\nsys.path.insert(0, {:?})\nfrom domain import *\n{script}",
            dir.display().to_string()
        ),
    )
    .unwrap();
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(&main)
        .output()
        .expect("python3 on PATH");
    std::fs::remove_dir_all(&dir).unwrap();
    out
}

/// buf generates every proto the reached ones import, used or not: a
/// pb2 module imports each of its file's imports (buy_in.proto imports
/// wallet.proto, none of whose messages the buy-in reaches).
#[test]
fn test_buf_generates_every_import_of_a_reached_proto() {
    let files = buy_in();
    let buf = source(&files, "buf.gen.yaml");
    for proto in [BUY_IN, "io/angzarr/examples/v1/wallet.proto"] {
        assert!(
            buf.contains(&format!("      - proto/{proto}\n")),
            "{proto}:\n{buf}"
        );
    }
    assert!(!buf.contains("io/angzarr/v1/"), "{buf}");
}

/// A projector naming its one source as `input_domain` reads that domain.
#[test]
fn test_a_projectors_input_domain_is_its_one_source() {
    const FILE: &str = "pyshop/v1/views.proto";
    let files = scaffold_python(
        &descriptors(&[FILE]),
        &[FILE],
        &app("projector-page-counter"),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let dispatch = source(&files, "_angzarr.py");
    assert!(
        flat(dispatch).contains("dispatch.for_domains(\"visits\")"),
        "{dispatch}"
    );
}

/// A map of an enum converts each value through the enum, a map of a
/// scalar as it is; a well-known type is never a plain type of the domain.
#[test]
fn test_a_map_converts_each_value_by_its_kind() {
    let files = till();
    let dispatch = source(&files, "till_state_angzarr.py");
    assert_has(
        dispatch,
        &[
            "drawers={k: _d.Drawer(v) for k, v in pb.drawers.items()},",
            "drawers={k: int(v) for k, v in value.drawers.items()},",
            "floats=dict(pb.floats)",
            "floats=dict(value.floats)",
        ],
    );
    let domain = source(&files, "/domain.py");
    assert!(
        domain.contains("    drawers: dict[str, Drawer] = field(default_factory=dict)\n"),
        "{domain}"
    );
    assert!(!domain.contains("class Timestamp"), "{domain}");
}

/// A policy whose target takes no command it can send types its
/// reactions' commands as any object.
#[test]
fn test_a_policy_whose_target_takes_facts_only_sends_any_object() {
    const FILE: &str = "pyshop/v1/views.proto";
    let files = scaffold_python(&descriptors(&[FILE]), &[FILE], &app("policy-pages-tally"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let logic = source(&files, "/logic.py");
    assert_has(logic, &["-> list[tuple[bytes, object]]: ..."]);
}

/// A projector's descriptor names its sources in its dispatch's order
/// (`input_domains`'), so the client's `describe` of the dispatch is the
/// same descriptor.
#[test]
fn test_a_projectors_descriptor_lists_its_sources_in_order() {
    let files = ledger();
    let descriptor = source(&files, "/_descriptor.py");
    let hex: String = descriptor
        .lines()
        .filter_map(|l| l.strip_prefix("        \"")?.strip_suffix('"'))
        .collect();
    let input = |domain: &str| {
        format!(
            "1a{:02x}0a{:02x}{}",
            domain.len() + 2,
            domain.len(),
            hex_of(domain)
        )
    };
    assert!(
        hex.contains(&format!("{}{}", input("table"), input("wallet"))),
        "{hex}"
    );
}
