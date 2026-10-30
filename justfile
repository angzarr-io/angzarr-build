# Build, test and lint the workspace. Plain cargo works too; these recipes are
# the checks CI and the git hooks (lefthook.yml) run.

# Submodule-protection recipes (install-submodule-hooks,
# check-submodules-clean). Source of truth: angzarr-project/submodule.just.
import? 'angzarr-project/submodule.just'

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
# Mutation-test the staged change, by hand (unjudged). No git hook runs
# mutation: it runs while tests are written and judged, at the end of an
# unattended run, and in the scheduled full pass (`mutants-all`).
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

# Mutation kill-rate thresholds, percent, by crate path: a mutant is judged by
# the longest prefix its file is under, and one under none fails the gate, so
# a new crate gets a threshold before it gets code. angzarr-codegen is at 100:
# its close-out runs reached 0 missed. The others are proposed at 90, the bar
# the org's other Rust repositories hold, until a full pass (`just
# mutants-all`) measures them.
mutation_thresholds := "crates/angzarr-codegen=100 crates/angzarr-build=90 crates/angzarr-cli=90 crates/angzarr-fmt=90"

# Mutation-test the whole workspace, judged by `mutation_thresholds`: the full
# pass, for a scheduled run (slow: a generated-code test builds and runs a
# project). mutants.out is target/mutants-all/mutants.out.
mutants-all:
    @just --justfile '{{justfile_directory()}}/justfile' _mutants-judge target/mutants-all

# lefthook's pre-push step, given the remote's name and git's pushed refs on
# stdin. For each ref pushed, in that commit's tree checked out alone
# (target/push-tree, so an unpushed edit cannot pass or fail it): every test
# of the workspace. No mutation: no git hook runs it. The range reported runs
# from the merge base of the commit pushed and the remote's current tip of
# that ref (a fast-forward's base is that tip); for a new ref, or a tip not
# fetched here, the merge base with the remote's default branch
# (<remote>/HEAD, else main). A deleted ref passes at once. The push tree's
# angzarr-project is the pushed commit's submodule commit, exported from this
# checkout's submodule (fetched if it is missing there).
#
# Red tests are allowed on a branch and kept out of the default branch: pushed
# to another branch, failing tests pass with a warning; pushed to the default
# branch they are refused. `lefthook` is what lefthook.yml passes to stop
# lefthook skipping it, and is ignored.
[positional-arguments]
push-check remote *lefthook:
    #!/usr/bin/env bash
    set -euo pipefail
    top='{{justfile_directory()}}'
    remote=$1
    default=$(git -C "$top" symbolic-ref --quiet --short "refs/remotes/$remote/HEAD" 2>/dev/null) && default=${default#"$remote"/} || default=main
    tree="$top/target/push-tree"
    mkdir -p "$top/target/logs"
    exec 9> "$top/target/.push-tree.lock"
    flock 9
    # git exports repository-locating variables (GIT_DIR, ...) into a hook;
    # with them, a git -C the push tree would act on this checkout instead,
    # and so would git run by a test there.
    unset $(git rev-parse --local-env-vars)
    pushed=0
    while read -r local_ref local_sha remote_ref remote_sha; do
        [ -n "${local_sha:-}" ] || continue
        [ "${local_sha//0/}" = "" ] && continue  # a deletion
        pushed=1
        target=${remote_ref#refs/heads/}
        base=
        if [ -n "${remote_sha//0/}" ] && git -C "$top" cat-file -e "$remote_sha^{commit}" 2>/dev/null; then
            base=$(git -C "$top" merge-base "$remote_sha" "$local_sha") || base=
        fi
        if [ -z "$base" ]; then
            for ref in "refs/remotes/$remote/$default" "refs/heads/$default"; do
                if git -C "$top" rev-parse --verify --quiet "$ref^{commit}" > /dev/null; then
                    base=$(git -C "$top" merge-base "$ref" "$local_sha") && break
                fi
            done
        fi
        if [ -z "$base" ]; then
            echo "FAIL push-check: $local_ref shares no history with $remote's tip or with $default" >&2
            exit 1
        fi
        range="$(git -C "$top" rev-parse --short "$base")..$(git -C "$top" rev-parse --short "$local_sha") to $remote/$target"
        # The pushed commit's tree, alone, no hook run for it.
        if ! git -C "$tree" rev-parse --git-dir > /dev/null 2>&1; then
            rm -rf "$tree"
            git -C "$top" worktree prune
            git -c core.hooksPath=/dev/null -C "$top" worktree add --quiet --detach "$tree" "$local_sha"
        fi
        git -c core.hooksPath=/dev/null -C "$tree" checkout --quiet --detach --force "$local_sha"
        git -C "$tree" clean -fdqx -e /target/
        sub=$(git -C "$top" ls-tree "$local_sha" angzarr-project | awk '{print $3}')
        if ! git -C "$top/angzarr-project" cat-file -e "$sub^{commit}" 2>/dev/null; then
            git -C "$top/angzarr-project" fetch --quiet origin "$sub" || {
                echo "FAIL push-check: angzarr-project $sub, the pushed commit's submodule, is not here (git submodule update --init)" >&2
                exit 1
            }
        fi
        rm -rf "$tree/angzarr-project"
        mkdir -p "$tree/angzarr-project"
        git -C "$top/angzarr-project" archive "$sub" | tar -x -C "$tree/angzarr-project"
        log="$top/target/logs/push-tests.log"
        rc=0
        (cd "$tree" && CARGO_TARGET_DIR="$top/target/push" cargo test --workspace) > "$log" 2>&1 9>&- || rc=$?
        if [ "$rc" -ne 0 ]; then
            if [ "$remote_ref" != "refs/heads/$default" ]; then
                echo "RED  push-check: $range: the tests fail (target/logs/push-tests.log); allowed off $default, which $target cannot merge into until they pass."
                continue
            fi
            echo "FAIL push-check: $range: the tests fail, and $default takes no red tests (full log: target/logs/push-tests.log)" >&2
            grep -E '^(test .* FAILED|failures:|error(\[|:))' "$log" | head -20 >&2 || true
            exit 1
        fi
        echo "ok   push-check ($range): tests pass"
    done
    [ "$pushed" = 1 ] || echo "ok   push-check (no commit pushed: nothing to test)"

# cargo-mutants over the workspace (or only the lines the diff PATCH changes,
# a/ and b/ paths relative to this tree), each mutant judged by the
# workspace's tests (a generated-code test builds and runs a project, so the
# timeout is generous), mutants.out under OUT; then each crate's kill rate
# against `mutation_thresholds`. Killed = caught or timed out; unviable
# mutants count neither way. Prints the rate per crate; exits 1 when one is
# below its threshold or a mutant's crate has none, listing the survivors.
[positional-arguments]
_mutants-judge out patch="":
    #!/usr/bin/env bash
    set -euo pipefail
    out=$1 patch=${2:-}
    if ! cargo mutants --version >/dev/null 2>&1; then
        echo "FAIL mutants: cargo-mutants is not installed; the gate cannot pass by not running." >&2
        echo "Install it: cargo install cargo-mutants --locked" >&2
        exit 1
    fi
    scope=(--workspace)
    [ -n "$patch" ] && scope+=(--in-diff "$patch")
    rm -rf "$out"
    mkdir -p "$out"
    rc=0
    cargo mutants "${scope[@]}" --test-workspace=true --in-place --timeout 300 \
        --output "$out" --colors never > "$out/cargo-mutants.log" 2>&1 || rc=$?
    case "$rc" in
        0|2|3) ;;
        *) echo "FAIL mutants: cargo mutants failed before a verdict (exit $rc; log: $out/cargo-mutants.log)" >&2; exit 1 ;;
    esac
    outcomes="$out/mutants.out/outcomes.json"
    judged='[.outcomes[] | select(.scenario | type == "object") | {file: .scenario.Mutant.file, summary}]'
    if [ ! -f "$outcomes" ] || [ "$(jq "$judged | length" "$outcomes")" -eq 0 ]; then
        echo "ok   mutants (no mutants: nothing mutable in scope)"
        exit 0
    fi
    tiered=$(jq --arg tiers '{{mutation_thresholds}}' "$judged"' | ($tiers | split(" ") | map(split("=") | {prefix: .[0], min: (.[1] | tonumber)})) as $t
        | map(. as $m | . + {tier: ([$t[] | . as $tier | select($m.file | startswith($tier.prefix + "/"))] | max_by(.prefix | length))})' "$outcomes")
    untiered=$(jq -r '.[] | select(.tier == null) | .file' <<< "$tiered" | sort -u)
    if [ -n "$untiered" ]; then
        echo "FAIL mutants: no threshold in mutation_thresholds (justfile) for: $(paste -sd ' ' <<< "$untiered")" >&2
        exit 1
    fi
    report=$(jq -r 'group_by(.tier.prefix)[] | (map(select(.summary == "CaughtMutant" or .summary == "Timeout")) | length) as $killed
        | (map(select(.summary == "MissedMutant")) | length) as $missed | ($killed + $missed) as $viable
        | .[0].tier as $tier | (if $viable == 0 then 100 else ($killed * 100 / $viable | floor) end) as $rate
        | "\(if $killed * 100 >= $tier.min * $viable then "pass" else "FAIL" end)  \($tier.prefix): \($killed)/\($viable) killed (\($rate)%), threshold \($tier.min)%"
            + (map(select(.summary == "Unviable")) | length | if . > 0 then "; \(.) unviable, not counted" else "" end)' <<< "$tiered")
    sed 's/^/  /' <<< "$report"
    if grep -q '^FAIL' <<< "$report"; then
        echo "FAIL mutants: below the kill-rate threshold (log: $out/cargo-mutants.log); surviving mutants:" >&2
        sed 's/^/  /' "$out/mutants.out/missed.txt" >&2 || true
        echo "Add or sharpen the test that would kill each survivor, commit it, and push again." >&2
        exit 1
    fi

# Wire lefthook's git hooks (lefthook.yml) into the repository's .git/hooks:
# once per clone (worktrees share them), and again when lefthook.yml names a
# new hook.
hooks-install:
    lefthook install

# lefthook's pre-commit tests: every test of the workspace. Red tests (the
# tests build and some fail) are committed on a branch other than main, with
# a warning that it cannot merge until they pass, and refused on main or a
# detached HEAD; a build failure is refused everywhere.
test-commit:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test --workspace --no-run --quiet
    cargo test --workspace && exit 0
    branch=$(git symbolic-ref --quiet --short HEAD) || branch=
    if [ -n "$branch" ] && [ "$branch" != main ]; then
        echo "RED  tests: red tests committed on $branch, which is allowed off main; $branch cannot merge into main (or be pushed to it) until they pass"
        exit 0
    fi
    echo "FAIL tests: red tests cannot be committed on ${branch:-a detached HEAD}. Commit them on a branch." >&2
    exit 1
