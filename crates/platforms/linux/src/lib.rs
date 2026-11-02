//! Linux 平台：Rockchip 官方 upgrade_tool 适配器 + Unix 进程与日志实现。
//!
//! 适配器本身与操作系统无关，可在任何平台编译和测试；Unix 宿主与装配入口只在 Unix 上编译。

#[cfg(unix)]
mod host;
mod parser;
mod upgrade_tool;

pub use upgrade_tool::UpgradeToolAdapter;

#[cfg(unix)]
pub use host::UnixHost;

/// 装配 Linux 版应用：发现 upgrade_tool，并以 Unix 宿主运行。
#[cfg(unix)]
pub fn create_application() -> rkflash_application::FlashApplication {
    use rkflash_runtime::{assemble_application, discover_tool, ToolBackend};
    use std::path::Path;
    use std::sync::Arc;

    let tool_path = discover_tool(
        "RK_UPGRADE_TOOL",
        "upgrade_tool",
        &[Path::new("/usr/lib/rkflash/upgrade_tool")],
    );
    assemble_application(
        ToolBackend {
            adapter: UpgradeToolAdapter,
            tool_path,
            tool_log_relative_path: UpgradeToolAdapter::runtime_log_relative_path(),
        },
        Arc::new(UnixHost),
    )
}
