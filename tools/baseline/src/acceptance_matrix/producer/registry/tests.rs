use super::{
    sensitive_env_name, valid_env_name, valid_identifier, validate_producer,
    validate_shell_invocation, validate_step,
};
use crate::acceptance_matrix::producer::model::{Producer, ProducerArg, ProducerStep};
use crate::acceptance_matrix::receipt_limits::MAX_EXECUTION_RECEIPTS;
use std::collections::BTreeMap;
use std::path::Path;

fn producer() -> Producer {
    Producer {
        producer_id: "test.producer".into(),
        evidence_ids: vec!["test.evidence".into()],
        tiers: vec!["ci".into()],
        host_os: vec!["windows".into()],
        timeout_seconds: 1,
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
fn producer_rejects_an_oversized_atomic_evidence_group() {
    let mut producer = producer();
    producer.evidence_ids = (0..=MAX_EXECUTION_RECEIPTS)
        .map(|index| format!("test.evidence-{index}"))
        .collect();

    assert!(validate_producer(Path::new("."), &producer).is_err());
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
