//! RKFlash 通用运行时：外部工具的串行执行、设备轮询、操作日志与工作区持久化。
//!
//! 本 crate 不含任何平台条件编译和烧录工具细节：操作系统差异通过 [`OsHost`] 注入，
//! 工具命令与输出解析通过应用层的 `ToolProtocol` 注入，两者都由 `crates/platforms/*` 提供。

mod assembly;
mod command;
mod discovery;
mod host;
mod logging;
mod monitor;
mod persistence;
mod runtime;
mod terminal;

pub use assembly::{assemble_application, ToolBackend};
pub use discovery::discover_tool;
pub use host::{LocalTime, OsHost, ToolProcessContext};
pub use persistence::{JsonConfigurationRepository, LocalFileSystem};
pub use runtime::ProcessRuntime;
