//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::read_workflow;
use super::text::{
    has_v_tag_trigger, require_exact_mapping_keys, require_ordered_text, require_text,
    yaml_mapping_block,
};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const WORKFLOW: &str = "android-emulator-admission.yml";
const VARIANTS: [(&str, &str); 3] = [
    ("pinned-api37-swangle", "swangle"),
    ("pinned-api37-software", "software"),
    ("pinned-api37-swiftshader", "swiftshader"),
];

pub(super) fn check(root: &Path) -> Result<()> {
    let workflow = read_workflow(root, WORKFLOW)?;
    validate_workflow(&workflow)?;
    let worker = read(root, "scripts/diagnose-android-emulator-admission.sh")?;
    let result_support = read(root, "scripts/lib/android-admission-diagnostic-result.sh")?;
    let lifecycle = read(root, "scripts/lib/android-admission-emulator-lifecycle.sh")?;
    let renderer = read(root, "scripts/lib/android-emulator-renderer.sh")?;
    let summary = read(root, "scripts/android-emulator-admission-summary.mjs")?;
    validate_worker(&worker, &result_support, &lifecycle, &renderer, &summary)
}

fn read(root: &Path, relative: &str) -> Result<String> {
    fs::read_to_string(root.join(relative)).with_context(|| format!("read {relative}"))
}

fn require_count(content: &str, needle: &str, expected: usize, scope: &str) -> Result<()> {
    let observed = content.matches(needle).count();
    if observed == expected {
        Ok(())
    } else {
        bail!(
            "release-baseline-check: expected {expected} occurrences of '{needle}' in {scope}, found {observed}"
        )
    }
}

fn matrix_entries(content: &str) -> Result<Vec<BTreeMap<String, String>>> {
    let matrix = yaml_mapping_block(content, 6, "matrix")?;
    let include = yaml_mapping_block(&matrix, 8, "include")?;
    let mut entries = Vec::new();
    let mut current = BTreeMap::new();
    for line in include.lines().skip(1) {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (starts_entry, field) = if let Some(field) = line.strip_prefix("          - ") {
            (true, field)
        } else if let Some(field) = line.strip_prefix("            ") {
            if field.starts_with(' ') {
                bail!("release-baseline-check: nested Android admission matrix value");
            }
            (false, field)
        } else {
            bail!("release-baseline-check: malformed Android admission matrix entry");
        };
        if starts_entry && !current.is_empty() {
            entries.push(std::mem::take(&mut current));
        }
        let (key, value) = field
            .split_once(':')
            .context("release-baseline-check: malformed Android admission matrix field")?;
        if key.is_empty()
            || value.trim().is_empty()
            || current.insert(key.into(), value.trim().into()).is_some()
        {
            bail!("release-baseline-check: invalid Android admission matrix field {key}");
        }
    }
    if !current.is_empty() {
        entries.push(current);
    }
    Ok(entries)
}

fn validate_workflow(content: &str) -> Result<()> {
    let on = yaml_mapping_block(content, 0, "on")?;
    require_exact_mapping_keys(&on, 2, &["workflow_dispatch"], WORKFLOW)?;
    let permissions = yaml_mapping_block(content, 0, "permissions")?;
    require_exact_mapping_keys(&permissions, 2, &["contents"], WORKFLOW)?;
    require_text(&permissions, "contents: read", WORKFLOW)?;
    require_count(content, "permissions:", 1, WORKFLOW)?;
    let jobs = yaml_mapping_block(content, 0, "jobs")?;
    require_exact_mapping_keys(&jobs, 2, &["admission", "build-apk", "summarize"], WORKFLOW)?;

    if has_v_tag_trigger(content) {
        bail!("release-baseline-check: Android admission diagnostic must not listen to v* tags");
    }
    for forbidden in [
        "secrets.",
        "contents: write",
        "acceptance-run",
        "--receipt-dir",
        "release-candidate.yml",
        "acceptance-aggregate.yml",
        "ANDROID_KEYSTORE",
    ] {
        if content.contains(forbidden) {
            bail!(
                "release-baseline-check: Android admission diagnostic contains forbidden '{forbidden}'"
            );
        }
    }

    require_count(content, "ref: ${{ github.sha }}", 3, WORKFLOW)?;
    require_count(
        content,
        "scripts/check-mobile-android-shell-package-build.sh",
        1,
        WORKFLOW,
    )?;
    require_count(
        content,
        "bash scripts/diagnose-android-emulator-admission.sh",
        1,
        WORKFLOW,
    )?;
    let entries = matrix_entries(content)?;
    if entries.len() != VARIANTS.len() {
        bail!(
            "release-baseline-check: expected {} Android admission matrix entries, found {}",
            VARIANTS.len(),
            entries.len()
        );
    }
    for ((variant, gpu_mode), entry) in VARIANTS.into_iter().zip(entries) {
        let expected = BTreeMap::from([
            ("api_level".to_owned(), "\"37.0\"".to_owned()),
            ("emulator_source".to_owned(), "pinned".to_owned()),
            ("gpu_mode".to_owned(), gpu_mode.to_owned()),
            ("variant".to_owned(), variant.to_owned()),
        ]);
        if entry != expected {
            bail!(
                "release-baseline-check: Android admission matrix entry {variant} drifted: {entry:?}"
            );
        }
    }
    require_count(content, "emulator_source: pinned", 3, WORKFLOW)?;
    require_count(content, "api_level: \"37.0\"", 3, WORKFLOW)?;
    require_text(
        content,
        "DEVE_ANDROID_ADMISSION_GPU_MODE: ${{ matrix.gpu_mode }}",
        WORKFLOW,
    )?;
    require_text(content, "timeout-minutes: 120", WORKFLOW)?;
    require_text(content, "DEVE_ANDROID_ADMISSION_CYCLES: \"3\"", WORKFLOW)?;
    require_text(content, "--expected-cycles \"3\"", WORKFLOW)?;
    require_text(content, "/cycle-[0-9]*", WORKFLOW)?;
    require_text(content, "continue-on-error: true", WORKFLOW)?;
    require_text(content, "fail-fast: false", WORKFLOW)?;
    require_text(content, "max-parallel: 3", WORKFLOW)?;
    require_text(content, "GITHUB_RUN_ATTEMPT", WORKFLOW)?;
    require_text(
        content,
        "node scripts/android-emulator-admission-summary.mjs",
        WORKFLOW,
    )?;
    require_ordered_text(
        content,
        &[
            "Build exact x86_64 diagnostic APK once",
            "Upload exact diagnostic APK",
            "admission:",
            "Run bounded cold-boot admission cycles",
            "Upload bounded variant result",
            "summarize:",
            "Download complete admission matrix",
            "Validate matrix and recommend the least-divergent stable variant",
        ],
        WORKFLOW,
    )
}

fn validate_worker(
    worker: &str,
    result_support: &str,
    lifecycle: &str,
    renderer: &str,
    summary: &str,
) -> Result<()> {
    for expected in [
        "source \"$ROOT_DIR/scripts/lib/android-emulator-owner.sh\"",
        "source \"$ROOT_DIR/scripts/lib/android-emulator-boot-readiness.sh\"",
        "source \"$ROOT_DIR/scripts/lib/android-emulator-renderer.sh\"",
        "source \"$ROOT_DIR/scripts/lib/android-install-retry.sh\"",
        "source \"$ROOT_DIR/scripts/lib/android-admission-diagnostic-result.sh\"",
        "source \"$ROOT_DIR/scripts/lib/android-admission-emulator-lifecycle.sh\"",
        "install_apk",
        "android_emulator_wait_for_guest_services_stable",
        "\"$system_pid_before\" == \"$system_pid_after\"",
        "    set +e\n    run_cycle \"$cycle\" >\"$RESULT_DIR/cycle-$cycle/cycle.log\" 2>&1\n    cycle_status=$?\n    set -e",
        "DEVE_ANDROID_ADMISSION_CYCLES:-3",
        "cold-boot cycles must be exactly 3",
        "swangle | software | swiftshader",
        "-gpu \"$GPU_MODE\"",
        "android_emulator_renderer_wait",
        "android_emulator_renderer_observe \"$cycle_dir/emulator.log\"",
        "rendererPair: (if $rendererPair == \"\" then null else $rendererPair end)",
    ] {
        require_text(worker, expected, "Android admission worker")?;
    }
    if worker.contains("if run_cycle ") {
        bail!(
            "release-baseline-check: Android admission worker must not suppress cycle errexit through a conditional function invocation"
        );
    }
    require_ordered_text(
        worker,
        &[
            "-gpu \"$GPU_MODE\"",
            "android_emulator_renderer_wait",
            "cycle_phase=\"boot-admission\"",
            "wait_for_cycle_boot",
        ],
        "Android admission worker",
    )?;
    require_ordered_text(
        worker,
        &[
            "android_admission_cleanup_emulator",
            "android_emulator_renderer_observe \"$cycle_dir/emulator.log\"",
            "completed_at=",
        ],
        "Android admission renderer finalization",
    )?;
    if worker.contains("adb install") || worker.contains(" install -r ") {
        bail!(
            "release-baseline-check: Android admission worker must use the shared install recovery boundary"
        );
    }
    for expected in [
        "head -c 65536",
        "ANDROID_ADMISSION_LOG_FILE_BUDGET_BYTES=131072",
        "ANDROID_ADMISSION_VARIANT_LOG_BUDGET_BYTES=4194304",
        "android_admission_verify_variant_log_budget",
        "logcat -b system -b crash",
        "dumpsys activity services",
        "android-emulator-admission-diagnostic",
        "gpuMode: $gpuMode",
        "renderer_identity",
    ] {
        require_text(
            result_support,
            expected,
            "Android admission diagnostic result support",
        )?;
    }
    for expected in [
        "launch_state=\"reserved\"",
        "jobs -pr",
        "cleanup-mobile-android-emulator.sh",
        "timeout --signal=TERM --kill-after=5s 45s",
        "kill -KILL",
    ] {
        require_text(lifecycle, expected, "Android admission emulator lifecycle")?;
    }
    for expected in [
        "android_emulator_renderer_observe()",
        "android_emulator_renderer_wait()",
        "ANDROID_EMULATOR_RENDERER_LAST_MODE=\"$modes\"",
        "android_emulator_renderer_verify()",
        "android_emulator_renderer_observe \"$log_file\" || return 1",
    ] {
        require_text(renderer, expected, "Android emulator renderer support")?;
    }
    for expected in [
        "expectedCycles === 3",
        "result.harnessError === null",
        "entry.systemServerPidBefore === entry.systemServerPidAfter",
        "matrix APK identity drifted across variants",
        "result.gpuMode === variant.gpuMode",
        "entry.rendererPair",
        "renderer identity drifted across cycles",
        "result.observedRendererPair !== control.observedRendererPair",
        "pinned emulator identity drifted across renderer variants",
        "API 37 system-image identity drifted across renderer variants",
        "recommendedVariantId",
        "not acceptance receipts",
    ] {
        require_text(summary, expected, "Android admission summary")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_worker, validate_workflow};

    const VALID: &str =
        include_str!("../../../../../.github/workflows/android-emulator-admission.yml");
    const WORKER: &str =
        include_str!("../../../../../scripts/diagnose-android-emulator-admission.sh");
    const RESULT_SUPPORT: &str =
        include_str!("../../../../../scripts/lib/android-admission-diagnostic-result.sh");
    const LIFECYCLE: &str =
        include_str!("../../../../../scripts/lib/android-admission-emulator-lifecycle.sh");
    const RENDERER: &str = include_str!("../../../../../scripts/lib/android-emulator-renderer.sh");
    const SUMMARY: &str =
        include_str!("../../../../../scripts/android-emulator-admission-summary.mjs");

    #[test]
    fn admission_workflow_is_manual_and_diagnostic_only() {
        validate_workflow(VALID).expect("valid Android admission workflow");

        for invalid in [
            VALID.replace("workflow_dispatch:", "push:"),
            VALID.replace(
                "permissions:\n  contents: read",
                "permissions:\n  contents: read\n  actions: write",
            ),
            VALID.replace(
                "  build-apk:\n",
                "  build-apk:\n    permissions:\n      packages: write\n",
            ),
            VALID.replace("continue-on-error: true", "continue-on-error: false"),
            VALID.replace("          - variant: pinned-api37-software\n", ""),
            VALID.replace(
                "            gpu_mode: software",
                "            gpu_mode: swangle",
            ),
            VALID.replace(
                "            gpu_mode: software",
                "            gpu_mode: software\n            ram_mb: 2048",
            ),
            format!("{VALID}\n# secrets.ANDROID_KEYSTORE\n"),
        ] {
            assert!(
                validate_workflow(&invalid).is_err(),
                "invalid admission workflow unexpectedly passed"
            );
        }
    }

    #[test]
    fn admission_worker_rejects_conditional_cycle_invocation() {
        validate_worker(WORKER, RESULT_SUPPORT, LIFECYCLE, RENDERER, SUMMARY)
            .expect("valid Android admission worker");

        let invalid = WORKER.replace(
            "    set +e\n    run_cycle ",
            "    set +e\n    if run_cycle ",
        );
        assert!(
            validate_worker(&invalid, RESULT_SUPPORT, LIFECYCLE, RENDERER, SUMMARY).is_err(),
            "conditional cycle invocation unexpectedly passed"
        );
    }
}
