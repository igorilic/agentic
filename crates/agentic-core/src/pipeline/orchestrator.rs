use tokio::task::JoinHandle;

use crate::db::runs::RunRepo;
use crate::db::steps::StepRepo;
use crate::events::EventBus;

/// Background orchestrator consuming events from the bus and applying
/// them to `runs`/`run_steps` rows.
pub struct PipelineOrchestrator;

impl PipelineOrchestrator {
    pub fn spawn(_bus: EventBus, _runs: RunRepo, _steps: StepRepo) -> JoinHandle<()> {
        tokio::spawn(async move {
            unimplemented!("PipelineOrchestrator::spawn not yet implemented")
        })
    }
}
