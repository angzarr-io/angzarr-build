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

# Clippy with warnings denied, and the rustfmt check.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

# Apply rustfmt.
fmt:
    cargo fmt --all
