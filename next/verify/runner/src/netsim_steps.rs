use anyhow::Result;

use crate::{
    orchestrator::TestContext,
    types::{ClientParams, Throughput},
};

pub struct NetsimWorld {}

impl NetsimWorld {
    pub async fn run_client(&mut self, _params: ClientParams) -> Result<Option<Throughput>> {
        Ok(None)
    }

    pub async fn reset_actor(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn get_label(&self) -> String {
        "netsim".to_string()
    }

    pub fn set_silent(&self, _s: bool) {}
}

/// STEP: Given @netsim is running
async fn netsim_running(w: &mut TestContext) {
    w.log_step("@netsim", "GIVEN", "Is running");
}

// Include generated glue code
include!(env!("NETSIM_STEPS_GLUE"));
