//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Result, bail};
use std::env;

const LABEL: &str = "docker-smoke-preflight";

pub fn run(args: &[String]) -> Result<()> {
    match args {
        [target] if target == "release" => run_release(target),
        [target] if target == "multiclient" => run_multiclient(target),
        [target] if target == "p2p-mesh" => run_p2p_mesh(target),
        [] => bail!("{LABEL}: expected docker smoke target: release, multiclient, or p2p-mesh"),
        [target] => bail!("{LABEL}: unsupported docker smoke target: {target}"),
        _ => bail!("{LABEL}: expected one docker smoke target"),
    }
}

fn run_release(target: &str) -> Result<()> {
    docker_switch_from_env("DEVE_DOCKER_SMOKE_REQUIRED", "0")?;
    port_from_env("DEVE_DOCKER_SMOKE_PORT", "3102")?;
    optional_unicode("DEVE_DOCKER_BIN")?;
    optional_unicode("DEVE_DOCKER_SMOKE_IMAGE")?;
    optional_unicode("DEVE_DOCKER_SMOKE_CONTAINER")?;
    optional_unicode("DEVE_DOCKER_SMOKE_AUTH_SECRET")?;
    optional_unicode("DEVE_DOCKER_SMOKE_AUTH_USER")?;
    optional_unicode("DEVE_DOCKER_SMOKE_AUTH_PASS")?;
    optional_unicode("DEVE_DOCKER_SMOKE_AUTH_PASSWORD")?;
    optional_unicode("DEVE_DOCKER_SMOKE_DATA_VOLUME")?;
    optional_unicode("DEVE_DOCKER_SMOKE_NOTES_VOLUME")?;
    ok(target);
    Ok(())
}

fn run_multiclient(target: &str) -> Result<()> {
    docker_switch_from_env("DEVE_DOCKER_MULTI_REQUIRED", "0")?;
    docker_switch_from_env("DEVE_DOCKER_MULTI_KEEP", "0")?;
    display_switch_from_env("DEVE_DOCKER_MULTI_HEADLESS", "1")?;
    port_from_env("DEVE_DOCKER_MULTI_PORT", "3101")?;
    positive_integer_from_env("DEVE_DOCKER_MULTI_TIMEOUT_MS", "60000")?;
    optional_unicode("DEVE_DOCKER_BIN")?;
    optional_unicode("DEVE_DOCKER_MULTI_COMPOSE_FILE")?;
    optional_unicode("DEVE_DOCKER_MULTI_PROJECT")?;
    optional_unicode("DEVE_DOCKER_MULTI_BASE_URL")?;
    optional_unicode("DEVE_DOCKER_MULTI_AUTH_SECRET")?;
    optional_unicode("DEVE_DOCKER_MULTI_AUTH_USER")?;
    optional_unicode("DEVE_DOCKER_MULTI_AUTH_PASS")?;
    optional_unicode("DEVE_DOCKER_MULTI_AUTH_PASSWORD")?;
    optional_unicode("DEVE_DOCKER_MULTI_PLAYWRIGHT_PACKAGE")?;
    optional_unicode("DEVE_DOCKER_MULTI_PLAYWRIGHT_WORK_DIR")?;
    optional_unicode("DEVE_DOCKER_MULTI_NODE_SCRIPT")?;
    ok(target);
    Ok(())
}

fn run_p2p_mesh(target: &str) -> Result<()> {
    docker_switch_from_env("DEVE_DOCKER_P2P_MESH_REQUIRED", "0")?;
    docker_switch_from_env("DEVE_DOCKER_P2P_MESH_KEEP", "0")?;
    docker_switch_from_env("DEVE_DOCKER_P2P_MESH_BUILDKIT", "0")?;
    docker_switch_from_env("DEVE_DOCKER_P2P_MESH_COMPOSE_DOCKER_CLI_BUILD", "0")?;
    positive_integer_from_env("DEVE_DOCKER_P2P_MESH_COMPOSE_PARALLEL_LIMIT", "1")?;
    let port_a = port_from_env("DEVE_DOCKER_P2P_MESH_A_PORT", "3111")?;
    let port_b = port_from_env("DEVE_DOCKER_P2P_MESH_B_PORT", "3112")?;
    if port_a == port_b {
        bail!("{LABEL}: DEVE_DOCKER_P2P_MESH_A_PORT and DEVE_DOCKER_P2P_MESH_B_PORT must differ");
    }
    exact_ascii_len_from_env(
        "DEVE_DOCKER_P2P_MESH_REPO_KEY",
        "deve_mesh_shared_repo_key_32!!!!",
        32,
    )?;
    optional_unicode("DEVE_DOCKER_BIN")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_COMPOSE_FILE")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_PROJECT")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_REPO_ID")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_AUTH_SECRET")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_AUTH_USER")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_AUTH_PASS")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_AUTH_PASSWORD")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_TOKEN_A")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_TOKEN_B")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_PEER_A_ID")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_PEER_B_ID")?;
    optional_unicode("DEVE_DOCKER_P2P_MESH_PYTHON_BIN")?;
    ok(target);
    Ok(())
}

fn ok(target: &str) {
    println!("{LABEL}: {target}: ok");
}

fn docker_switch_from_env(name: &str, default: &str) -> Result<bool> {
    parse_docker_switch(name, &env_value_or(name, default)?)
}

fn display_switch_from_env(name: &str, default: &str) -> Result<bool> {
    parse_display_switch(name, &env_value_or(name, default)?)
}

fn parse_docker_switch(name: &str, value: &str) -> Result<bool> {
    match value {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        _ => bail!("{LABEL}: {name} must be 0, 1, true, or false"),
    }
}

fn parse_display_switch(name: &str, value: &str) -> Result<bool> {
    match value {
        "0" | "false" | "no" => Ok(false),
        "1" | "true" | "yes" => Ok(true),
        _ => bail!("{LABEL}: {name} must be 0, 1, true, false, yes, or no"),
    }
}

fn port_from_env(name: &str, default: &str) -> Result<u16> {
    let value = env_value_or(name, default)?;
    match value.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => bail!("{LABEL}: {name} must be a TCP port in 1..=65535"),
    }
}

fn positive_integer_from_env(name: &str, default: &str) -> Result<u64> {
    let value = env_value_or(name, default)?;
    if matches!(value.as_bytes().first(), Some(b'1'..=b'9'))
        && value.as_bytes().iter().all(u8::is_ascii_digit)
    {
        return value
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("{LABEL}: {name} must be a positive integer"));
    }
    bail!("{LABEL}: {name} must be a positive integer");
}

fn exact_ascii_len_from_env(name: &str, default: &str, expected: usize) -> Result<()> {
    let value = env_value_or(name, default)?;
    if value.is_ascii() && value.len() == expected {
        return Ok(());
    }
    bail!("{LABEL}: {name} must be exactly {expected} ASCII bytes");
}

fn optional_unicode(name: &str) -> Result<()> {
    match env::var(name) {
        Ok(_) | Err(env::VarError::NotPresent) => Ok(()),
        Err(env::VarError::NotUnicode(_)) => bail!("{LABEL}: {name} must be valid Unicode"),
    }
}

fn env_value_or(name: &str, default: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) if value.is_empty() => Ok(default.to_string()),
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_string()),
        Err(env::VarError::NotUnicode(_)) => bail!("{LABEL}: {name} must be valid Unicode"),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_display_switch, parse_docker_switch};

    #[test]
    fn docker_switch_matches_shell_required_and_keep_flags() {
        for value in ["0", "false"] {
            assert!(!parse_docker_switch("DEVE_FLAG", value).expect("flag"));
        }
        for value in ["1", "true"] {
            assert!(parse_docker_switch("DEVE_FLAG", value).expect("flag"));
        }
    }

    #[test]
    fn docker_switch_rejects_ambiguous_values() {
        for value in ["", "yes", "TRUE", "2", "maybe"] {
            assert!(parse_docker_switch("DEVE_FLAG", value).is_err());
        }
    }

    #[test]
    fn display_switch_accepts_playwright_headless_values() {
        for value in ["0", "false", "no"] {
            assert!(!parse_display_switch("DEVE_DOCKER_MULTI_HEADLESS", value).expect("flag"));
        }
        for value in ["1", "true", "yes"] {
            assert!(parse_display_switch("DEVE_DOCKER_MULTI_HEADLESS", value).expect("flag"));
        }
    }
}
