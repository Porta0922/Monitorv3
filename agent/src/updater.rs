//! Remote self-update is unavailable until signed platform releases and rollback are validated.
//! Local administrative updates live in packaging/{linux,windows}/update-service.*.
#[derive(Debug)]
pub enum ApplyUpdateResult {
    UpToDate,
    Updated { version: String },
    Failed(String),
}

pub async fn apply_update() -> ApplyUpdateResult {
    ApplyUpdateResult::Failed("Automatic legacy updates are disabled. Use the platform lifecycle package; signed release updates are pending.".into())
}
