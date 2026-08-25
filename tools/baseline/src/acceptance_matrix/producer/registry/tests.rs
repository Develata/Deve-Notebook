use super::{
    artifact_location_allowed, sensitive_env_name, valid_env_name, valid_identifier,
    validate_dependencies, validate_producer, validate_shell_invocation, validate_step,
};
use crate::acceptance_matrix::producer::model::{
    Producer, ProducerArg, ProducerRegistry, ProducerStep,
};
use crate::acceptance_matrix::receipt_limits::MAX_EXECUTION_RECEIPTS;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn producer() -> Producer {
    Producer {
        producer_id: "test.producer".into(),
        evidence_ids: vec!["test.evidence".into()],
        dependencies: Vec::new(),
        tiers: vec!["ci".into()],
        host_os: vec!["windows".into()],
        timeout_seconds: 1,
        required_tools: Vec::new(),
        required_env: Vec::new(),
        bound_env: Vec::new(),
        environment: BTreeMap::new(),
        claims_env: BTreeMap::new(),
        artifacts: vec!["scripts/test.sh".into()],
        steps: Vec::new(),
        finally_steps: Vec::new(),
        note: "test".into(),
    }
}

#[test]
fn identifiers_and_environment_names_are_narrow() {
    assert!(valid_identifier("docker.multiclient-v1"));
    assert!(!valid_identifier("Docker command"));
    assert!(valid_env_name("DEVE_RECEIPT_1"));
    assert!(!valid_env_name("deve_receipt"));
    assert!(sensitive_env_name("DEVE_REMOTE_PASSWORD"));
    assert!(sensitive_env_name("DEVE_API_KEY"));
    assert!(!sensitive_env_name("DEVE_RELEASE_CANDIDATE_IMAGE_ID"));
}

#[test]
fn direct_node_steps_require_explicit_tool_metadata() {
    let mut producer = producer();
    producer.artifacts.clear();
    producer.steps.push(ProducerStep {
        program: "node".into(),
        args: vec![ProducerArg::LiteralString("--version".into())],
    });
    let error = validate_producer(Path::new("."), &producer, &BTreeMap::new())
        .expect_err("undeclared Node tool requirement must fail closed")
        .to_string();
    assert!(
        error.contains("without declaring required_tools node"),
        "{error}"
    );

    producer.required_tools.push("node".into());
    validate_producer(Path::new("."), &producer, &BTreeMap::new())
        .expect("declared Node tool requirement");
}

#[test]
fn shell_invocation_rejects_command_string_variants() {
    let producer = producer();
    for option in ["-c", "-lc", "-xc", "--command"] {
        let args = vec![ProducerArg::Literal {
            literal: option.into(),
        }];
        assert!(validate_shell_invocation(&producer, "bash", &args).is_err());
    }
    for option in ["/C", "/K"] {
        let args = vec![ProducerArg::Literal {
            literal: option.into(),
        }];
        assert!(validate_shell_invocation(&producer, "cmd.exe", &args).is_err());
    }
    for option in ["-Command", "-EncodedCommand", "-Co", "-Ec"] {
        let args = vec![ProducerArg::Literal {
            literal: option.into(),
        }];
        assert!(validate_shell_invocation(&producer, "pwsh", &args).is_err());
    }
}

#[test]
fn shell_invocation_accepts_direct_script_files() {
    let producer = producer();
    let bash = vec![ProducerArg::Literal {
        literal: "scripts/test.sh".into(),
    }];
    assert!(validate_shell_invocation(&producer, "bash", &bash).is_ok());
    let powershell = ["-NoProfile", "-File", "scripts/test.ps1"]
        .into_iter()
        .map(|literal| ProducerArg::Literal {
            literal: literal.into(),
        })
        .collect::<Vec<_>>();
    assert!(validate_shell_invocation(&producer, "pwsh.exe", &powershell).is_ok());
}

#[test]
fn producer_artifacts_admit_only_scripts_or_root_compose_manifests() {
    assert!(artifact_location_allowed("scripts/hostname"));
    assert!(artifact_location_allowed(
        "docker-compose.remote-import.yml"
    ));
    assert!(!artifact_location_allowed("compose.yml"));
    assert!(!artifact_location_allowed(
        "scripts/../docker-compose.remote-import.yml"
    ));
}

#[test]
fn producer_rejects_an_oversized_atomic_evidence_group() {
    let mut producer = producer();
    producer.evidence_ids = (0..=MAX_EXECUTION_RECEIPTS)
        .map(|index| format!("test.evidence-{index}"))
        .collect();

    assert!(validate_producer(Path::new("."), &producer, &BTreeMap::new()).is_err());
}

#[test]
fn producer_rejects_sensitive_environment_in_process_arguments() {
    let mut producer = producer();
    producer.required_env.push("DEVE_REMOTE_PASSWORD".into());
    let step = ProducerStep {
        program: "deve-test".into(),
        args: vec![ProducerArg::Env {
            env: "DEVE_REMOTE_PASSWORD".into(),
        }],
    };

    assert!(validate_step(Path::new("."), &producer, &step).is_err());
}

#[test]
fn ci_cargo_test_steps_require_a_narrow_target() {
    let producer = producer();
    let broad = ProducerStep {
        program: "cargo".into(),
        args: ["test", "--locked", "-p", "deve_core", "selector"]
            .into_iter()
            .map(|literal| ProducerArg::LiteralString(literal.into()))
            .collect(),
    };
    assert!(validate_step(Path::new("."), &producer, &broad).is_err());

    let mut narrow = broad;
    narrow
        .args
        .insert(4, ProducerArg::LiteralString("--lib".into()));
    assert!(validate_step(Path::new("."), &producer, &narrow).is_ok());
}

#[test]
fn producer_dependencies_reject_cycles() {
    let mut one = producer();
    one.producer_id = "test.one".into();
    one.dependencies = vec!["test.two".into()];
    let mut two = producer();
    two.producer_id = "test.two".into();
    two.dependencies = vec!["test.one".into()];
    let registry = ProducerRegistry {
        schema: 2,
        producers: vec![one, two],
    };
    let ids = BTreeSet::from(["test.one", "test.two"]);

    let error = validate_dependencies(&registry, &ids).unwrap_err();

    assert!(error.to_string().contains("dependency cycle"));
}
