//! RKFlash 前后端唯一契约：只包含稳定类型和能力接口，不含任何实现。
//!
//! 每个模块对应前端的一个功能区，前端 `desktop/actions/<模块>.rs` 只调用同名模块的接口，
//! 后端应用层的 `service/<模块>.rs` 实现同名接口：
//!
//! | 模块 | 接口 | 前端功能区 |
//! |---|---|---|
//! | `device` | [`DeviceApi`] | 顶栏设备状态 |
//! | `task` | [`TaskApi`] | 任务进度、取消 |
//! | `download` | [`DownloadApi`] | 下载镜像页 |
//! | `upgrade` | [`UpgradeApi`] | 升级固件页 |
//! | `advanced` | [`AdvancedApi`] | 高级功能页 |
//! | `settings` | [`SettingsApi`] | 烧录工具与工具日志目录 |
//! | `log` | [`LogApi`] | 操作日志 |
//! | `workspace` | [`WorkspaceApi`] | 配置导入导出 |
//!
//! 会改变设备的操作都成对提供 `check_*`（确认前校验）与执行函数。

mod advanced;
mod device;
mod download;
mod log;
mod settings;
mod task;
mod upgrade;
mod workspace;

pub use advanced::{
    AdvancedApi, DeviceControl, DeviceQuery, LbaEraseRequest, LbaReadRequest, LbaWriteRequest,
    LoaderUpgradeRequest, SerialLogRequest,
};
pub use device::{DeviceApi, DeviceInfo, DeviceSnapshot, StorageType};
pub use download::{DownloadApi, DownloadRequest};
pub use log::LogApi;
pub use settings::{SettingsApi, ToolInfo};
pub use task::{ExecutionEvent, TaskApi};
pub use upgrade::{EraseFlashRequest, FirmwareUpgradeRequest, UpgradeApi};
pub use workspace::{ImageEntry, WorkspaceApi, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};

/// 前端持有的完整后端能力：各功能区接口的组合，任何实现了全部接口的类型自动满足。
pub trait FlashService:
    DeviceApi + TaskApi + DownloadApi + UpgradeApi + AdvancedApi + SettingsApi + LogApi + WorkspaceApi
{
}

impl<T> FlashService for T where
    T: DeviceApi
        + TaskApi
        + DownloadApi
        + UpgradeApi
        + AdvancedApi
        + SettingsApi
        + LogApi
        + WorkspaceApi
{
}
