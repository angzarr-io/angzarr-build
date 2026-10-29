//! The Python scaffold: one domain's uv project, the user's logic
//! interface and implementation written once over the domain's plain types,
//! the plain types, the dispatch and the harness glue the CLI's. Pinned by golden files (`tests/golden/python/player`), the
//! blackjack player of spec decision 14.38.

mod common;

use angzarr_codegen::{
    CodegenError, Owner, ProtoInput, PythonApp, ScaffoldFile, complete, python_reroots,
    reroot_python, scaffold_python,
};
use common::*;

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/python/player");

fn player_app() -> PythonApp {
    PythonApp {
        domain: "player".to_string(),
        protos: ProtoInput::Git {
            url: "https://github.com/angzarr-io/angzarr-project".to_string(),
            rev: "a91b88262921786c246e63e2e694509fe8060a86".to_string(),
            subdir: "proto".to_string(),
        },
        rules_version: "0.1.0".to_string(),
    }
}

/// The player's project, from player.proto alone: the rest of the
/// blackjack protos are in the descriptor set (a policy and a projector
/// name the player's events), not scaffolded.
fn player() -> Vec<ScaffoldFile> {
    scaffold_python(&descriptors(BLACKJACK), &[PLAYER], &player_app()).unwrap()
}

fn file<'a>(files: &'a [ScaffoldFile], path: &str) -> &'a ScaffoldFile {
    files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no {path}"))
}

const IMPL: &str = "src/angzarr_player/logic_impl.py";
const INTERFACE: &str = "src/angzarr_player/logic.py";

#[test]
fn test_scaffold_python_writes_the_projects_files_each_with_its_owner() {
    let mut placed: Vec<(String, Owner)> =
        player().into_iter().map(|f| (f.path, f.owner)).collect();
    placed.sort_by(|a, b| a.0.cmp(&b.0));
    let gen_dir = "src/angzarr_player/_gen";
    let expected: Vec<(String, Owner)> = [
        ("buf.gen.yaml", Owner::Generated),
        ("pyproject.toml", Owner::User),
        ("src/angzarr_player/__init__.py", Owner::User),
        ("src/angzarr_player/__main__.py", Owner::Generated),
        ("src/angzarr_player/domain.py", Owner::Generated),
        (&format!("{gen_dir}/__init__.py"), Owner::Generated),
        (&format!("{gen_dir}/io/__init__.py"), Owner::Generated),
        (
            &format!("{gen_dir}/io/angzarr/__init__.py"),
            Owner::Generated,
        ),
        (
            &format!("{gen_dir}/io/angzarr/examples/__init__.py"),
            Owner::Generated,
        ),
        (
            &format!("{gen_dir}/io/angzarr/examples/v1/__init__.py"),
            Owner::Generated,
        ),
        (
            &format!("{gen_dir}/io/angzarr/examples/v1/player_aggregate_angzarr.py"),
            Owner::Generated,
        ),
        ("src/angzarr_player/_version.py", Owner::Generated),
        ("src/angzarr_player/main.py", Owner::Generated),
        (INTERFACE, Owner::User),
        (IMPL, Owner::User),
        ("tests/__init__.py", Owner::User),
    ]
    .iter()
    .map(|(p, o)| (p.to_string(), *o))
    .collect();
    let mut expected = expected;
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(placed, expected);
}

#[test]
fn test_scaffold_python_matches_the_golden_project() {
    for f in player() {
        let golden = std::fs::read_to_string(format!("{GOLDEN}/{}", f.path))
            .unwrap_or_else(|_| panic!("no golden {}", f.path));
        assert_eq!(f.source, golden, "{}", f.path);
    }
}

#[test]
fn test_scaffold_python_pins_the_client_by_the_one_constant() {
    let files = player();
    let pyproject = &file(&files, "pyproject.toml").source;
    assert!(
        pyproject.contains(&format!(
            "angzarr-client = {{ git = \"{}\", rev = \"{}\" }}",
            angzarr_codegen::CLIENT_PYTHON_GIT,
            angzarr_codegen::CLIENT_PYTHON_REV
        )),
        "{pyproject}"
    );
}

#[test]
fn test_the_users_files_name_nothing_of_the_harness() {
    // pyproject.toml depends on angzarr-client and protobuf; the user's code
    // names neither.
    for f in player()
        .iter()
        .filter(|f| f.owner == Owner::User && f.path.ends_with(".py"))
    {
        for harness in [
            "angzarr_client",
            "ComponentHost",
            "_az",
            "run_child",
            "_pb2",
            "protobuf",
        ] {
            assert!(
                !f.source.contains(harness),
                "{} names {harness}:\n{}",
                f.path,
                f.source
            );
        }
        // It imports the plain types and its own interface, nothing generated
        // from protobuf.
        for import in f
            .source
            .lines()
            .filter(|l| l.starts_with("from ") || l.starts_with("import "))
        {
            assert!(!import.contains("_gen"), "{}: {import}", f.path);
        }
    }
}

#[test]
fn test_scaffold_python_refuses_a_domain_the_protos_do_not_declare() {
    let app = PythonApp {
        domain: "nowhere".to_string(),
        ..player_app()
    };
    assert_eq!(
        scaffold_python(&descriptors(BLACKJACK), &[PLAYER], &app),
        Err(CodegenError::PythonNoDomain {
            domain: "nowhere".to_string()
        })
    );
}

// ------------------------------------------------------------- --missing

/// The implementation as a user has it: the decision written, the fold not
/// there yet.
const USER_IMPL: &str = r#""""The player bot."""

from __future__ import annotations

from angzarr_player.domain import (
    DecideMove,
    MoveAction,
    MoveDecided,
    PlayerState,
)
from angzarr_player.logic import PlayerLogic


class PlayerLogicImpl(PlayerLogic):
    def decide_move(self, cmd: DecideMove, state: PlayerState) -> MoveDecided:
        action = MoveAction.HIT if cmd.total < 17 else MoveAction.STAND
        return MoveDecided(table_root=cmd.table_root, round=cmd.round, seat=cmd.seat, action=action)  # USER-WRITTEN


def helper() -> int:
    return 1
"#;

#[test]
fn test_complete_adds_only_the_absent_methods_to_the_class() {
    let fresh = file(&player(), IMPL).clone();
    let completed = complete(&fresh, USER_IMPL)
        .unwrap()
        .expect("apply_move_decided is absent");
    // The method goes in at the end of the class body, and nothing of the
    // user's text changes.
    let expected = USER_IMPL.replace(
        "  # USER-WRITTEN\n",
        "  # USER-WRITTEN\n\n    def apply_move_decided(self, state: PlayerState, event: MoveDecided) \
         -> PlayerState:\n        raise NotImplementedError  # TODO: fold MoveDecided into the state\n",
    );
    assert_eq!(completed, expected);
    let helper = completed.find("def helper").unwrap();
    assert!(completed.find("def apply_move_decided").unwrap() < helper);
}

#[test]
fn test_complete_of_a_complete_python_file_changes_nothing() {
    for path in [IMPL, INTERFACE] {
        let fresh = file(&player(), path).clone();
        assert_eq!(complete(&fresh, &fresh.source), Ok(None), "{path}");
    }
}

#[test]
fn test_complete_refuses_a_command_taken_as_a_fact() {
    let fresh = file(&player(), INTERFACE).clone();
    let as_fact = fresh.source.replace("cmd: DecideMove", "fact: DecideMove");
    assert_ne!(as_fact, fresh.source);
    assert_eq!(
        complete(&fresh, &as_fact),
        Err(CodegenError::InputMismatch {
            path: INTERFACE.to_string(),
            message: "io.angzarr.examples.v1.DecideMove".to_string(),
            expected: "cmd".to_string(),
            found: "fact".to_string(),
        })
    );
}

#[test]
fn test_complete_refuses_a_python_file_without_the_class() {
    let fresh = file(&player(), IMPL).clone();
    assert_eq!(
        complete(&fresh, "\"\"\"Not the player any more.\"\"\"\n"),
        Err(CodegenError::ScaffoldImplNotFound {
            path: IMPL.to_string(),
            component: "io.angzarr.examples.v1.PlayerState".to_string(),
        })
    );
}

// --------------------------------------------------------------- reroot

#[test]
fn test_reroot_python_moves_framework_and_domain_imports_under_their_packages() {
    let rules = python_reroots(&player_app(), &["io.angzarr.examples.v1"]);
    let pb2 = "from google.protobuf import descriptor as _descriptor\n\
               from io.angzarr.examples.v1 import cards_pb2 as io_dot_angzarr_dot_examples_dot_v1_dot_cards__pb2\n\
               from io.angzarr.v1 import options_pb2 as io_dot_angzarr_dot_v1_dot_options__pb2\n\
               import io.angzarr.v1.types_pb2\n\
               x = 'from io.angzarr.v1 import y'\n";
    assert_eq!(
        reroot_python(pb2, &rules),
        "from google.protobuf import descriptor as _descriptor\n\
         from angzarr_player._gen.io.angzarr.examples.v1 import cards_pb2 as io_dot_angzarr_dot_examples_dot_v1_dot_cards__pb2\n\
         from angzarr_client.proto.io.angzarr.v1 import options_pb2 as io_dot_angzarr_dot_v1_dot_options__pb2\n\
         import angzarr_client.proto.io.angzarr.v1.types_pb2\n\
         x = 'from io.angzarr.v1 import y'\n"
    );
}

#[test]
fn test_the_logic_takes_and_returns_the_domains_plain_types() {
    let files = player();
    let logic = &file(&files, INTERFACE).source;
    assert!(logic.contains("class PlayerLogic(Protocol):"), "{logic}");
    assert!(
        logic.contains(
            "    def decide_move(self, cmd: DecideMove, state: PlayerState) -> MoveDecided: ..."
        ),
        "a decision: the command and the state in, the event out:\n{logic}"
    );
    assert!(
        logic.contains("self, state: PlayerState, event: MoveDecided\n    ) -> PlayerState: ..."),
        "a fold: the state and the event in, the next state out:\n{logic}"
    );
    let domain = &file(&files, "src/angzarr_player/domain.py").source;
    for plain in [
        "class MoveAction(enum.IntEnum):",
        "    DOUBLE_DOWN = 3",
        "@dataclass(frozen=True)\nclass DecideMove:",
        "    dealer_up: Card | None = None",
        "    cards: tuple[Card, ...] = ()",
        "    action: MoveAction = MoveAction.UNSPECIFIED",
    ] {
        assert!(domain.contains(plain), "{plain}:\n{domain}");
    }
}
