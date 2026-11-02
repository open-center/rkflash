//! RKFlash 后端：应用用例、设备领域状态和 Rockchip 基础设施适配器。

mod application;
pub mod domain;
mod infrastructure;

use application::{ApplicationPorts, FlashPlanner, FlashProfile, ToolProtocol};
use infrastructure::{
    discover_rkdeveloptool, discover_upgrade_tool, JsonConfigurationRepository, LocalFileSystem,
    ProcessRuntime, RkDevelopToolAdapter, UpgradeToolAdapter,
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
    if uses_rkdeveloptool() {
        assemble(
            RkDevelopToolAdapter::default,
            discover_rkdeveloptool(),
            RkDevelopToolAdapter::runtime_log_relative_path(),
        )
    } else {
        assemble(
            UpgradeToolAdapter::default,
            discover_upgrade_tool(),
            UpgradeToolAdapter::runtime_log_relative_path(),
        )
    }
}

/// Windows 固定使用开源 rkdeveloptool；其他平台可用 `RKFLASH_BACKEND=rkdeveloptool` 调试该后端。
fn uses_rkdeveloptool() -> bool {
    cfg!(windows)
        || std::env::var("RKFLASH_BACKEND").is_ok_and(|backend| backend == "rkdeveloptool")
}

fn assemble<A>(
    adapter: impl Fn() -> A,
    tool_path: Option<std::path::PathBuf>,
    tool_log_relative_path: std::path::PathBuf,
) -> FlashApplication
where
    A: FlashPlanner + FlashProfile + ToolProtocol + 'static,
{
    // 协议解析在后台线程运行，需要 Arc + Send/Sync；
    // 计划与分区表只在 UI 线程使用，适配器无状态，直接各建一份 Rc 实例。
    let protocol: Arc<dyn ToolProtocol> = Arc::new(adapter());
    let planner: Rc<dyn FlashPlanner> = Rc::new(adapter());
    let profile: Rc<dyn FlashProfile> = Rc::new(adapter());
    let runtime = Rc::new(ProcessRuntime::new(
        tool_path,
        protocol,
        tool_log_relative_path,
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
