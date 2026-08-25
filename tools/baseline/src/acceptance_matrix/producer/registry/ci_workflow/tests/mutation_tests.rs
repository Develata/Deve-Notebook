//! Adversarial mutation coverage for artifact and fan-in boundaries.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{VALID, error};

#[test]
fn rejects_path_poisoning_or_any_extra_run_step() {
    let poisoned = VALID.replace(
        "      - run: cargo fetch --locked\n      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.shared --producer ci.linux",
        "      - run: cargo fetch --locked\n      - run: echo /tmp/fake-cargo >>\"$GITHUB_PATH\"\n      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.shared --producer ci.linux",
    );
    assert_ne!(poisoned, VALID);
    assert!(error(&poisoned).contains("only fetch"));

    let base_poisoned = VALID.replacen(
        "      - run: cargo fetch --locked",
        "      - run: cargo fetch --locked\n      - run: echo poisoned >>\"$GITHUB_ENV\"",
        1,
    );
    assert_ne!(base_poisoned, VALID);
    assert!(error(&base_poisoned).contains("command sequence"));
}

#[test]
fn rejects_non_shadow_uploads_and_formal_artifact_names() {
    let extra_upload = VALID.replace(
        "  check:\n    if: ${{ always() }}",
        "  stray-upload:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/upload-artifact@v7\n        with:\n          name: diagnostic\n          path: target/example\n  check:\n    if: ${{ always() }}",
    );
    assert!(error(&extra_upload).contains("may not upload artifacts"));

    let formal_name = VALID.replace(
        "name: deve-impact-shadow-${{ github.sha }}",
        "name: deve-acceptance-receipts-${{ github.sha }}",
    );
    assert!(error(&formal_name).contains("formal artifact prefix"));
}

#[test]
fn rejects_an_unobserved_extra_job_even_without_artifacts() {
    let extra_job = VALID.replace(
        "  check:\n    if: ${{ always() }}",
        "  unobserved-failure:\n    runs-on: ubuntu-latest\n    steps:\n      - run: exit 1\n  check:\n    if: ${{ always() }}",
    );
    assert!(error(&extra_job).contains("job set mismatch"));
}

#[test]
fn rejects_fan_in_that_can_hide_non_success() {
    let conditional = VALID.replace("if: ${{ always() }}", "if: false");
    assert!(error(&conditional).contains("must be a string"));

    let missing_need = VALID.replace("      - watcher-native-fs\n", "");
    assert!(error(&missing_need).contains("needs mismatch"));

    let weak_condition = VALID.replace(
        "needs.watcher-native-fs.result != 'success'",
        "needs.watcher-native-fs.result == 'failure'",
    );
    assert!(error(&weak_condition).contains("does not reject every non-success"));

    let inert = VALID.replace("        run: exit 1", "        run: true");
    assert!(error(&inert).contains("must be a string"));

    let tolerated = VALID.replace(
        "        run: exit 1",
        "        continue-on-error: true\n        run: exit 1",
    );
    let tolerated_error = error(&tolerated);
    assert!(
        tolerated_error.contains("may not tolerate fan-in failure"),
        "{tolerated_error}"
    );

    let custom_shell = VALID.replace(
        "        run: exit 1",
        "        shell: echo {0}\n        run: exit 1",
    );
    assert!(error(&custom_shell).contains("may not declare shell"));

    let fan_in_defaults = VALID.replace(
        "  check:\n    if: ${{ always() }}",
        "  check:\n    defaults:\n      run:\n        shell: echo {0}\n    if: ${{ always() }}",
    );
    assert!(error(&fan_in_defaults).contains("may not declare defaults"));

    let tolerated_dependency = VALID.replace(
        "  contract-checks:\n    runs-on: ubuntu-latest",
        "  contract-checks:\n    continue-on-error: true\n    runs-on: ubuntu-latest",
    );
    assert!(error(&tolerated_dependency).contains("required fan-in dependency failure"));

    let skipped_dependency_step = VALID.replacen(
        "      - run: |\n          cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan",
        "      - if: false\n        run: |\n          cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan",
        1,
    );
    assert!(error(&skipped_dependency_step).contains("may not declare if"));
}
