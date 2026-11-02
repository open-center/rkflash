//! Windows 平台：开源 rkdeveloptool 适配器 + Win32 进程与日志实现。
//!
//! 适配器本身与操作系统无关，可在任何平台编译和测试；Win32 宿主与装配入口只在 Windows 上编译。

#[cfg(windows)]
mod host;
mod rkdeveloptool;

pub use rkdeveloptool::RkDevelopToolAdapter;

#[cfg(windows)]
pub use host::WindowsHost;

/// 装配 Windows 版应用：rkdeveloptool.exe 随安装包放在 rkflash.exe 同目录。
#[cfg(windows)]
pub fn create_application() -> rkflash_application::FlashApplication {
    use rkflash_runtime::{assemble_application, discover_tool, ToolBackend};
    use std::sync::Arc;

    assemble_application(
        ToolBackend {
            adapter: RkDevelopToolAdapter,
            tool_path: discover_tool("RK_DEVELOP_TOOL", "rkdeveloptool.exe", &[]),
            tool_log_relative_path: RkDevelopToolAdapter::runtime_log_relative_path(),
        },
        Arc::new(WindowsHost),
    )
}
