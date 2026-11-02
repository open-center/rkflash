mod execution;
mod sector;

pub use execution::{CommandSpec, ExecutionPlan, ToolOutcome, ToolSession, Workflow};
pub use rkflash_api::{
    DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot, ExecutionEvent, FlashOperation,
    StorageType,
};
pub use sector::normalize_sector;
