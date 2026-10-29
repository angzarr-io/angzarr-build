# Build, test and lint the workspace. Plain cargo works too; these recipes are
# the checks CI and pre-commit run.

# List the recipes.
default:
    @just --list

# Build everything, tests included.
build:
    cargo build --workspace --all-targets

# Run every test.
test:
    cargo test --workspace

# `just test` runs every package's; this is the quick loop on one crate.
# Arguments after a `--` go to the test binaries, as with cargo test: `just
# test-pkg angzarr-build -- --ignored local`.
# Run the named packages' tests only.
test-pkg +args:
    #!/usr/bin/env bash
    set -euo pipefail
    set -- {{args}}
    packages=()
    while [ "$#" -gt 0 ] && [ "$1" != -- ]; do packages+=(-p "$1"); shift; done
    exec cargo test "${packages[@]}" "$@"

# cargo-mutants over the mutants the staged Rust diff touches, each judged
# by the workspace's tests. A generated-code test builds and runs a project,
# so a test may take minutes: the timeout is generous so a slow test is not
# taken for a hung mutant.
# Mutation-test the staged change.
mutants-staged:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! cargo mutants --version >/dev/null 2>&1; then
        echo "mutants-staged: cargo-mutants is not installed; the gate cannot pass by not running." >&2
        echo "Install it: cargo install cargo-mutants --locked" >&2
        exit 1
    fi
    mkdir -p target
    patch=target/mutants-staged.patch
    git diff --cached HEAD -- '*.rs' > "$patch"
    if [ ! -s "$patch" ]; then
        echo "mutants-staged: the staged change touches no Rust source; nothing to mutate."
        exit 0
    fi
    exec cargo mutants --workspace --test-workspace=true --in-diff "$patch" \
        --timeout 300 --output target/mutants-staged --colors never

# Clippy with warnings denied, and the rustfmt check.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

# Apply rustfmt.
fmt:
    cargo fmt --all
