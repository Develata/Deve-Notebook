//! Cache and required base-job adversarial cases.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{VALID, error};

fn replace_section(source: &str, start: &str, end: &str, replacement: &str) -> String {
    let (prefix, rest) = source.split_once(start).expect("fixture section start");
    let (_, suffix) = rest.split_once(end).expect("fixture section end");
    format!("{prefix}{replacement}{end}{suffix}")
}

#[test]
fn rejects_missing_or_broad_required_job_cache() {
    let target_cache = VALID.replacen(
        "            ~/.cargo/git/db",
        "            ~/.cargo/git/db\n            target",
        1,
    );
    assert!(error(&target_cache).contains("source-only Cargo cache"));

    let broad_key = VALID.replacen(
        "${{ runner.os }}-cargo-source-rust-1.97.0-${{ hashFiles('Cargo.lock') }}",
        "${{ runner.os }}-cargo-source-${{ hashFiles('**/Cargo.lock') }}",
        1,
    );
    assert!(error(&broad_key).contains("exact source-only Cargo cache key"));

    let missing_cache = VALID.replacen(
        "      - uses: actions/cache@v6\n        with:\n          path: |\n            ~/.cargo/registry/index\n            ~/.cargo/registry/cache\n            ~/.cargo/registry/src\n            ~/.cargo/git/db\n          key: ${{ runner.os }}-cargo-source-rust-1.97.0-${{ hashFiles('Cargo.lock') }}\n          restore-keys: ${{ runner.os }}-cargo-source-rust-1.97.0-\n",
        "",
        1,
    );
    assert!(error(&missing_cache).contains("exactly one source-only Cargo cache"));

    let alternate_restore = VALID.replacen(
        "      - run: cargo fetch --locked",
        "      - uses: actions/cache/restore@v4\n        with:\n          path: target\n          key: target-cache\n      - run: cargo fetch --locked",
        1,
    );
    assert!(error(&alternate_restore).contains("unsupported action"));

    let third_party_cache = VALID.replacen(
        "      - run: cargo fetch --locked",
        "      - uses: Swatinem/rust-cache@v2\n      - run: cargo fetch --locked",
        1,
    );
    assert!(error(&third_party_cache).contains("unsupported action"));

    let incomplete_seed = VALID.replacen("cargo fetch --locked", "cargo metadata --locked", 1);
    assert!(error(&incomplete_seed).contains("first command must be exact"));
}

#[test]
fn rejects_noop_or_cross_wired_base_jobs_and_watcher_drift() {
    let noop_tests = VALID.replacen("      - run: cargo test --locked", "      - run: true", 1);
    assert!(error(&noop_tests).contains("must be a string"));

    let name_only = VALID.replacen(
        "      - run: cargo clippy --locked --all-targets -- -D warnings",
        "      - name: cargo clippy --locked --all-targets -- -D warnings\n        run: cargo --version",
        1,
    );
    assert!(error(&name_only).contains("command sequence must exactly match"));

    let cross_wired = VALID.replacen(
        "      - run: cargo test --locked",
        "      - run: cargo clippy --locked --all-targets -- -D warnings",
        1,
    );
    assert!(error(&cross_wired).contains("command sequence must exactly match"));

    let watcher_host = VALID.replace("runs-on: ${{ matrix.os }}", "runs-on: ubuntu-latest");
    assert!(error(&watcher_host).contains("exact watcher OS matrix"));

    let watcher_axis = VALID.replace("os: [ubuntu-latest, windows-latest]", "os: [ubuntu-latest]");
    assert!(error(&watcher_axis).contains("exact Linux and Windows hosts"));

    let guard_name_only = replace_section(
        VALID,
        "      - run: |\n          forbidden=(",
        "      - run: cargo fmt --check",
        "      - name: check-only guard\n        run: true\n",
    );
    assert!(error(&guard_name_only).contains("must be a string"));

    let missing_plans = replace_section(
        VALID,
        "      - run: |\n          cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan",
        "      - run: node --test scripts/android-lifecycle-harness.test.mjs scripts/mobile-android-emulator-journey.test.mjs",
        "",
    );
    assert!(error(&missing_plans).contains("command sequence must exactly match"));

    let missing_coverage = replace_section(
        VALID,
        "      - run: |\n          scripts/plan-coverage.sh --check-reverse-coverage",
        "  rust-quality:",
        "",
    );
    assert!(error(&missing_coverage).contains("command sequence must exactly match"));
}
