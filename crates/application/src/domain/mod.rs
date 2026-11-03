mod execution;
mod operation;
mod sector;

pub use execution::{CommandSpec, ExecutionPlan};
pub use operation::FlashOperation;
pub use rkflash_api::{
    DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot, ExecutionEvent, StorageType, ToolInfo,
};
pub use rkflash_core::session::{ToolOutcome, ToolSession, Workflow};
pub use sector::normalize_sector;
