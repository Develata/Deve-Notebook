//! Indirect producer artifact host/tool contract validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::acceptance_matrix::producer::model::Producer;
use anyhow::{Result, bail};

const BOUNDED_UNIX_FIXTURE: &str = "scripts/lib/remote-browser-fixture-bounded.sh";
const BOUNDED_UNIX_TOOLS: [&str; 4] = ["bash", "node", "python3", "setsid"];

pub(super) fn validate_indirect_tool_contract(producer: &Producer) -> Result<()> {
    if !producer
        .artifacts
        .iter()
        .any(|artifact| artifact == BOUNDED_UNIX_FIXTURE)
    {
        return Ok(());
    }
    if producer.host_os.as_slice() != ["linux"] {
        bail!(
            "acceptance producers: {} bounded Unix fixture is restricted to host_os linux",
            producer.producer_id
        );
    }
    for required_tool in BOUNDED_UNIX_TOOLS {
        if !producer
            .required_tools
            .iter()
            .any(|tool| tool == required_tool)
        {
            bail!(
                "acceptance producers: {} bounded Unix fixture requires required_tools {required_tool}",
                producer.producer_id
            );
        }
    }
    Ok(())
}
