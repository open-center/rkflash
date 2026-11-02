mod execution;
mod sector;

pub use execution::{CommandSpec, ExecutionPlan};
pub use rkflash_api::{
    DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot, ExecutionEvent, FlashOperation,
    StorageType,
};
pub use rkflash_core::session::{ToolOutcome, ToolSession, Workflow};
pub use sector::normalize_sector;
