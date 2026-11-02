//! RKTools 前后端共享契约；只包含稳定类型和应用能力接口。

mod device;
mod execution;
mod operation;
mod service;

pub use device::{DeviceInfo, DeviceSnapshot};
pub use execution::{ApplicationUpdate, ExecutionEvent};
pub use operation::{DeviceControl, DeviceQuery, FlashOperation, StorageType};
pub use rktools_config::{ImageEntry, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};
pub use service::FlashService;
