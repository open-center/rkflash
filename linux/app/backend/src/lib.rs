//! RKFlash 后端：应用用例、设备领域状态和 Rockchip 基础设施适配器。

mod application;
pub mod domain;
mod infrastructure;

use application::{ApplicationPorts, FlashPlanner, FlashProfile, ToolProtocol};
use infrastructure::{
    discover_upgrade_tool, JsonConfigurationRepository, LocalFileSystem, ProcessRuntime,
    UpgradeToolAdapter,
};
use std::rc::Rc;
use std::sync::Arc;

pub use application::FlashApplication;
pub use domain::{
    DeviceControl, DeviceInfo, DeviceQuery, DeviceSnapshot, ExecutionEvent, FlashOperation,
    StorageType,
};
pub use rkflash_api::{
    ApplicationUpdate, FlashService, ImageEntry, WorkspaceConfig, WORKSPACE_FORMAT_VERSION,
};

pub fn create_application() -> FlashApplication {
    // 协议解析在后台线程运行，需要 Arc + Send/Sync；
    // 计划与分区表只在 UI 线程使用，适配器无状态，直接各建一份 Rc 实例。
    let protocol: Arc<dyn ToolProtocol> = Arc::new(UpgradeToolAdapter::default());
    let planner: Rc<dyn FlashPlanner> = Rc::new(UpgradeToolAdapter::default());
    let profile: Rc<dyn FlashProfile> = Rc::new(UpgradeToolAdapter::default());
    let runtime = Rc::new(ProcessRuntime::new(
        discover_upgrade_tool(),
        protocol,
        UpgradeToolAdapter::runtime_log_relative_path(),
    ));

    FlashApplication::new(ApplicationPorts {
        executor: runtime.clone(),
        device_monitor: runtime.clone(),
        tool_locator: runtime.clone(),
        operation_log: runtime,
        planner,
        profile,
        workspace_repository: Box::new(JsonConfigurationRepository),
        file_system: Box::new(LocalFileSystem),
    })
}
